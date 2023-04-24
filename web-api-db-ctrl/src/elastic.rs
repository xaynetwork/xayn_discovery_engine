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

use once_cell::sync::Lazy;
use reqwest::Method;
use serde_json::{json, Value};
use xayn_web_api_shared::{elastic::Client, request::TenantId};

use crate::Error;

static MAPPING_STR: &str = include_str!("../elastic-search/mapping.json");
static MAPPING: Lazy<Value> = Lazy::new(|| serde_json::from_str(MAPPING_STR).unwrap());

pub(super) async fn create_tenant(elastic: &Client, new_id: &TenantId) -> Result<(), Error> {
    elastic
        .with_index(new_id)
        .request(Method::PUT, [], [])
        .json(&*MAPPING)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

pub(super) async fn delete_tenant(elastic: &Client, tenant_id: &TenantId) -> Result<(), Error> {
    elastic
        .with_index(tenant_id)
        .request(Method::DELETE, [], [])
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

pub(crate) async fn setup_legacy_tenant(
    elastic: &Client,
    default_index: &str,
    tenant_id: &TenantId,
) -> Result<(), Error> {
    elastic
        .with_index("_aliases")
        .request(Method::POST, [], [])
        .json(&json!({
          "actions": [
            {
              "add": {
                "index": default_index,
                "alias": tenant_id,
              }
            }
          ]
        }))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
