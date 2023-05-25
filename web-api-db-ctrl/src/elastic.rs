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
use reqwest::{Method, StatusCode};
use serde_json::{json, Value};
use tracing::{error, info};
use xayn_web_api_shared::{elastic::Client, request::TenantId};

use crate::Error;

static MAPPING_STR: &str = include_str!("../elasticsearch/mapping.json");
static MAPPING: Lazy<Value> = Lazy::new(|| serde_json::from_str(MAPPING_STR).unwrap());

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

pub(crate) async fn migrate_tenant_index(
    elastic: &Client,
    tenant_id: &TenantId,
    embedding_size: usize,
) -> Result<(), Error> {
    if !does_tenant_index_exist(elastic, tenant_id).await? {
        error!(
            {%tenant_id},
            "index for tenant doesn't exist, creating a new index"
        );
        create_tenant_index(elastic, tenant_id, embedding_size).await?;
    }
    //FIXME code to check if the index is a super-set of the expected index
    //FIXME code for allowing updates to the ES schema, at least if
    //      incremental application of the change is possible
    Ok(())
}

async fn does_tenant_index_exist(
    elastic: &Client,
    tenant_id: impl AsRef<str>,
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

async fn create_index_alias(
    elastic: &Client,
    index: &str,
    alias: impl AsRef<str>,
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
    fn test_setting_embedding_dims_works() {
        assert_eq!(
            mapping_with_embedding_size(&MAPPING, 4321).unwrap(),
            json!({
                "mappings": {
                    "properties": {
                        "snippet": {
                            "type": "text"
                        },
                        "embedding": {
                            "type": "dense_vector",
                            "dims": 4321,
                            "index": true,
                            "similarity": "dot_product"
                        },
                        "properties": {
                            "dynamic": false,
                            "properties": {
                                "publication_date": {
                                    "type": "date"
                                }
                            }
                        }
                    }
                }
            })
        )
    }
}
