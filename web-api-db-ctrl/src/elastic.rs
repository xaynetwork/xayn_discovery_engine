// Copyright 2023 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::fmt::Debug;

use anyhow::bail;
use once_cell::sync::Lazy;
use reqwest::{Method, StatusCode};
use serde_json::{json, Value};
use tracing::{error, info, instrument};
use xayn_web_api_shared::{elastic::Client, request::TenantId};

use crate::Error;

static MAPPING_STR: &str = include_str!("../elasticsearch/mapping.json");
static MAPPING: Lazy<Value> = Lazy::new(|| serde_json::from_str(MAPPING_STR).unwrap());

#[instrument(skip(elastic))]
pub async fn create_tenant_index(
    elastic: &Client,
    new_id: &TenantId,
    embedding_size: usize,
) -> Result<(), Error> {
    let mapping = mapping_with_embedding_size(&MAPPING, embedding_size)?;
    elastic
        .with_index(new_id)
        .request(Method::PUT, [], [])
        .json(&mapping)
        .send()
        .await?
        .error_for_status()?;
    info!({tenant_id = %new_id}, "created ES index");
    Ok(())
}

#[instrument(skip(elastic))]
pub(super) async fn delete_tenant(elastic: &Client, tenant_id: &TenantId) -> Result<(), Error> {
    elastic
        .with_index(tenant_id)
        .request(Method::DELETE, [], [])
        .send()
        .await?
        .error_for_status()?;
    info!({%tenant_id}, "deleted ES index");
    Ok(())
}

#[instrument(skip(elastic))]
pub(crate) async fn setup_legacy_tenant(
    elastic: &Client,
    default_index: &str,
    tenant_id: &TenantId,
    embedding_size: usize,
) -> Result<(), Error> {
    if does_tenant_index_exist(elastic, default_index).await? {
        create_index_alias(elastic, default_index, tenant_id).await?;
    } else {
        create_tenant_index(elastic, tenant_id, embedding_size).await?;
    }

    Ok(())
}

#[instrument(skip(elastic))]
pub(crate) async fn migrate_tenant_index(
    elastic: &Client,
    tenant_id: &TenantId,
    embedding_size: usize,
) -> Result<(), Error> {
    if let Some(existing_mapping) = get_opt_tenant_mapping(elastic, tenant_id).await? {
        let base_mapping = mapping_with_embedding_size(&MAPPING, embedding_size)?;
        check_mapping_compatibility(&existing_mapping, &base_mapping)?;
    } else {
        error!(
            {%tenant_id},
            "index for tenant doesn't exist, creating a new index"
        );
        create_tenant_index(elastic, tenant_id, embedding_size).await?;
    }
    Ok(())
}

const MAPPINGS: &str = "mappings";
const PROPERTIES: &str = "properties";
const EMBEDDING: &str = "embedding";

fn check_mapping_compatibility(
    existing_mapping: &Value,
    base_mapping: &Value,
) -> Result<(), Error> {
    let existing_embeddings = &existing_mapping[MAPPINGS][PROPERTIES][EMBEDDING];
    let expected_embeddings = &base_mapping[MAPPINGS][PROPERTIES][EMBEDDING];
    if existing_embeddings != expected_embeddings {
        error!({ %existing_embeddings, %expected_embeddings }, "mappings in ES have incompatible embedding definition");
        bail!("incompatible existing elasticsearch index");
    }
    //FIXME add support for schema migrations
    // - check compatibility for all properties, not just embedding
    // - add new mapping (e.g. `tags`)
    // - handle settings which can be updated
    //   - e.g. ignore_malformed
    //FIXME
    // - cross check document.properties with indexed property schema
    //   - update ES if needed/possible
    //   - maybe move ES indexed property creation outside of pg commit time,
    //     which with this update step could be more robust wrt. some failures
    //      (mainly network timeout), but more problematic with other failure
    //      (bugs leading to things being in the ES schema but not in PG).
    Ok(())
}

#[instrument(skip(elastic))]
async fn does_tenant_index_exist(
    elastic: &Client,
    tenant_id: impl AsRef<str> + Debug,
) -> Result<bool, Error> {
    let response = elastic
        .with_index(tenant_id)
        .request(Method::HEAD, [], [])
        .send()
        .await?;

    let status = response.status();
    if status == StatusCode::NOT_FOUND {
        Ok(false)
    } else {
        response.error_for_status()?;
        Ok(true)
    }
}

#[instrument(skip(elastic))]
async fn get_opt_tenant_mapping(
    elastic: &Client,
    tenant_id: impl AsRef<str> + Debug,
) -> Result<Option<Value>, Error> {
    let response = elastic
        .with_index(&tenant_id)
        .request(Method::GET, ["_mapping"], [])
        .send()
        .await?;

    let status = response.status();
    if status == StatusCode::NOT_FOUND {
        Ok(None)
    } else {
        match response.error_for_status()?.json().await? {
            Value::Object(obj) if obj.len() == 1 => {
                Ok(obj.into_iter().next().map(|(_, mapping)| mapping))
            }
            response => bail!("unexpected index/_mapping response: {response}"),
        }
    }
}

#[instrument(skip(elastic))]
async fn create_index_alias(
    elastic: &Client,
    index: &str,
    alias: impl AsRef<str> + Debug,
) -> Result<(), Error> {
    let alias = alias.as_ref();
    elastic
        .with_index("_aliases")
        .request(Method::POST, [], [])
        .json(&json!({
            "actions": [
            {
                "add": {
                "index": index,
                "alias": alias,
                }
            }
            ]
        }))
        .send()
        .await?
        .error_for_status()?;
    info!({%index, %alias}, "created ES alias");
    Ok(())
}

fn mapping_with_embedding_size(mapping: &Value, embedding_size: usize) -> Result<Value, Error> {
    let mut mapping = mapping.clone();
    if let Some(dims) = mapping
        .get_mut("mappings")
        .and_then(|obj| obj.get_mut("properties"))
        .and_then(|obj| obj.get_mut("embedding"))
        .and_then(|obj| obj.get_mut("dims"))
    {
        *dims = embedding_size.into();
    } else {
        bail!("unexpected ES mapping structure can't set embedding.dims")
    }
    Ok(mapping)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creating_embedding_mapping_works() {
        let result = mapping_with_embedding_size(&MAPPING, 4321).unwrap();
        let embeddings = result
            .get("mappings")
            .and_then(|obj| obj.get("properties"))
            .and_then(|obj| obj.get("embedding"))
            .expect("path mappings.properties.embedding must be given");

        assert_eq!(
            embeddings,
            &json!({
                "type": "dense_vector",
                "dims": 4321,
                "index": true,
                "similarity": "dot_product"
            })
        );
    }

    #[test]
    fn test_snippet_has_a_mapping() {
        let result = mapping_with_embedding_size(&MAPPING, 128).unwrap();
        result
            .get("mappings")
            .and_then(|obj| obj.get("properties"))
            .and_then(|obj| obj.get("snippet"))
            .expect("path mappings.properties.snippet must be given");
    }

    #[test]
    fn test_properties_mapping_is_not_dynamic() {
        let result = mapping_with_embedding_size(&MAPPING, 128).unwrap();
        let dynamic_setting = result
            .get("mappings")
            .and_then(|obj| obj.get("properties"))
            .and_then(|obj| obj.get("properties"))
            .and_then(|obj| obj.get("dynamic"))
            .expect("path mappings.properties.properties.dynamic must be given");
        assert_eq!(dynamic_setting, &json!(false));
    }

    #[test]
    fn test_publication_date_is_mapped_is_correct() {
        let result = mapping_with_embedding_size(&MAPPING, 128).unwrap();
        let publication_date = result
            .get("mappings")
            .and_then(|obj| obj.get("properties"))
            .and_then(|obj| obj.get("properties"))
            .and_then(|obj| obj.get("properties"))
            .and_then(|obj| obj.get("publication_date"))
            .expect(
                "path mappings.properties.properties.properties.publication_date must be given",
            );
        assert_eq!(
            publication_date,
            &json!({ "type": "date", "ignore_malformed": true })
        );
    }
}
