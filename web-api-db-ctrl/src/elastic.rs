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

use anyhow::bail;
use once_cell::sync::Lazy;
use reqwest::Method;
use serde_json::{json, Value};
use tracing::{error, info, instrument};
use xayn_web_api_shared::elastic::{Client, ClientWithoutIndex, NotFoundAsOptionExt, SerdeDiscard};

use crate::{postgres::ExternalMigrator, tenant::Tenant, Error};

static MAPPING_STR: &str = include_str!("../elasticsearch/mapping.json");
static MAPPING: Lazy<Value> = Lazy::new(|| serde_json::from_str(MAPPING_STR).unwrap());

#[instrument(skip(elastic))]
pub async fn create_tenant_index(
    elastic: &ClientWithoutIndex,
    tenant: &Tenant,
    embedding_size: usize,
) -> Result<(), Error> {
    let elastic = elastic.with_index(&tenant.es_index_name);
    let mapping = mapping_with_embedding_size(&MAPPING, embedding_size)?;
    elastic
        .query_with_json::<_, SerdeDiscard>(Method::PUT, elastic.create_url([], []), Some(&mapping))
        .await?;
    info!("created ES index");
    Ok(())
}

#[instrument(skip(elastic))]
pub async fn delete_index(elastic: &ClientWithoutIndex, index_name: &str) -> Result<(), Error> {
    let elastic = elastic.with_index(index_name);
    elastic
        .query_with_bytes::<SerdeDiscard>(Method::DELETE, elastic.create_url([], []), None)
        .await?;
    info!({%index_name}, "deleted ES index");
    Ok(())
}

#[instrument(skip(elastic, migrator))]
pub(crate) async fn migrate_tenant_index(
    elastic: &ClientWithoutIndex,
    tenant: &Tenant,
    embedding_size: usize,
    migrator: &mut impl ExternalMigrator,
) -> Result<(), Error> {
    let es_with_index = elastic.with_index(&tenant.es_index_name);
    if let Some(existing_mapping) = get_opt_tenant_mapping(&es_with_index).await? {
        let base_mapping = mapping_with_embedding_size(&MAPPING, embedding_size)?;
        check_mapping_compatibility(&existing_mapping, &base_mapping)?;
    } else {
        error!(
            {%tenant.tenant_id},
            "index for tenant doesn't exist, creating a new index"
        );
        create_tenant_index(elastic, tenant, embedding_size).await?;
    }

    migrator
        .run_migration_if_needed("migrate_parent_property", async move {
            migrate_parent_property(&es_with_index).await
        })
        .await?;

    Ok(())
}

async fn migrate_parent_property(elastic: &Client) -> Result<(), Error> {
    let res = elastic
        .query_with_json::<_, Value>(
            Method::POST,
            elastic.create_url(
                ["_update_by_query"],
                [
                    ("refresh", None),
                    ("conflicts", Some("proceed")),
                    ("wait_for_completion", Some("false")),
                    (
                        "requests_per_second",
                        Some(&elastic.default_request_per_second().to_string()),
                    ),
                ],
            ),
            Some(json!({
                "script": {
                    "source": "ctx._source.putIfAbsent('parent', ctx._id)",
                },
            })),
        )
        .await?;

    if let Some(task_id) = res.get("task").and_then(|value| value.as_str()) {
        info!({%task_id}, "parent property migration running in background");
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
pub(crate) async fn does_index_exist(
    elastic: &ClientWithoutIndex,
    index: &str,
) -> Result<bool, Error> {
    Ok(get_opt_tenant_mapping(&elastic.with_index(index))
        .await?
        .is_some())
}

#[instrument(skip(elastic))]
async fn get_opt_tenant_mapping(elastic: &Client) -> Result<Option<Value>, Error> {
    let response = elastic
        .query_with_bytes::<Value>(Method::GET, elastic.create_url(["_mapping"], []), None)
        .await
        .not_found_as_option()?;
    match response {
        None => Ok(None),
        Some(Value::Object(obj)) if obj.len() == 1 => {
            Ok(obj.into_iter().next().map(|(_, mapping)| mapping))
        }
        Some(unexpected) => bail!("unexpected index/_mapping response: {unexpected}"),
    }
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
