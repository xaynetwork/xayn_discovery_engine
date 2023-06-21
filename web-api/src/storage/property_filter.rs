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

//! This module contains generic code used by the various aspects of property filtering.

use std::collections::HashMap;

use derive_more::{Display, From, IntoIterator};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::models::DocumentPropertyId;

#[derive(Debug, Display, PartialEq, Error, Serialize)]
#[non_exhaustive]

pub(crate) enum IncompatibleUpdate {
    #[display(fmt = "Property {property} is already defined.")]
    PropertyIsAlreadyIndexed { property: DocumentPropertyId },
    #[display(
        fmt = "Only {allowed} indexed properties including publication_date are allowed, got: {count}"
    )]
    TooManyProperties { count: usize, allowed: usize },
}

//Hint: Currently the API and internal definition match so we use the same type.
#[derive(Debug, Clone, Default, IntoIterator, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct IndexedPropertiesSchemaUpdate(
    #[into_iterator(owned, ref)] IndexedPropertiesSchema,
);

impl IndexedPropertiesSchemaUpdate {
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }
}

//Hint: Currently the API and internal definition match so we use the same type.
#[derive(Debug, Clone, Default, IntoIterator, From, Serialize, Deserialize)]
pub(crate) struct IndexedPropertiesSchema {
    #[into_iterator(owned, ref)]
    properties: HashMap<DocumentPropertyId, IndexedPropertyDefinition>,
}

impl IndexedPropertiesSchema {
    pub(crate) fn len(&self) -> usize {
        self.properties.len()
    }

    pub(crate) fn update(
        &mut self,
        schema_update: &IndexedPropertiesSchemaUpdate,
        max_total_properties: usize,
    ) -> Result<(), IncompatibleUpdate> {
        for (property, _) in schema_update {
            if self.properties.contains_key(property) {
                return Err(IncompatibleUpdate::PropertyIsAlreadyIndexed {
                    property: property.clone(),
                });
            }
        }
        let count = self.len() + schema_update.len();
        if count > max_total_properties {
            return Err(IncompatibleUpdate::TooManyProperties {
                count,
                allowed: max_total_properties,
            });
        }
        self.properties.extend(schema_update.clone().into_iter());
        Ok(())
    }
}

//Hint: Currently the API and internal definition match so we use the same type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct IndexedPropertyDefinition {
    pub(crate) r#type: IndexedPropertyType,
}

//Hint: Currently the API and internal definition match so we use the same type.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "indexed_property_type")]
#[sqlx(rename_all = "snake_case")]
pub(crate) enum IndexedPropertyType {
    Bool,
    Number,
    String,
    #[serde(rename = "string[]")]
    #[sqlx(rename = "string[]")]
    StringArray,
    Date,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    impl IndexedPropertiesSchema {
        fn from_iter<'a>(iter: impl IntoIterator<Item = (&'a str, IndexedPropertyType)>) -> Self {
            let properties = iter
                .into_iter()
                .map(|(name, r#type)| {
                    (
                        DocumentPropertyId::try_from(name).unwrap(),
                        IndexedPropertyDefinition { r#type },
                    )
                })
                .collect();
            Self { properties }
        }
    }

    impl IndexedPropertiesSchemaUpdate {
        fn from_iter<'a>(iter: impl IntoIterator<Item = (&'a str, IndexedPropertyType)>) -> Self {
            Self(IndexedPropertiesSchema::from_iter(iter))
        }
    }

    #[test]
    fn test_update_checking_works_with_no_overlap() {
        let mut schema = IndexedPropertiesSchema::from_iter([
            ("foo", IndexedPropertyType::String),
            ("bar", IndexedPropertyType::Number),
        ]);
        let update = IndexedPropertiesSchemaUpdate::from_iter([
            ("foot", IndexedPropertyType::String),
            ("bart", IndexedPropertyType::Number),
        ]);

        schema.update(&update, 11).expect("to be compatible");
    }

    #[test]
    fn test_update_checking_works_with_overlap() {
        let mut schema = IndexedPropertiesSchema::from_iter([
            ("foo", IndexedPropertyType::String),
            ("bar", IndexedPropertyType::Number),
        ]);
        let update = IndexedPropertiesSchemaUpdate::from_iter([
            ("foo", IndexedPropertyType::String),
            ("bart", IndexedPropertyType::Number),
        ]);

        let err = schema.update(&update, 11).unwrap_err();

        assert_eq!(
            err,
            IncompatibleUpdate::PropertyIsAlreadyIndexed {
                property: "foo".try_into().unwrap()
            }
        )
    }

    #[test]
    fn test_update_checks_max_properties() {
        let mut schema = IndexedPropertiesSchema::from_iter([("foo", IndexedPropertyType::String)]);
        let update = IndexedPropertiesSchemaUpdate::from_iter([
            ("bar", IndexedPropertyType::String),
            ("baz", IndexedPropertyType::String),
        ]);

        let err = schema.update(&update, 2).unwrap_err();

        assert_eq!(
            err,
            IncompatibleUpdate::TooManyProperties {
                count: 3,
                allowed: 2
            }
        );

        let mut schema = IndexedPropertiesSchema::from_iter([
            ("foo", IndexedPropertyType::String),
            ("bar", IndexedPropertyType::Number),
        ]);
        let update = IndexedPropertiesSchemaUpdate::from_iter([]);

        let err = schema.update(&update, 1).unwrap_err();

        assert_eq!(
            err,
            IncompatibleUpdate::TooManyProperties {
                count: 2,
                allowed: 1
            }
        );
    }

    #[test]
    fn test_type_serialization() {
        let value = json!([
            IndexedPropertyType::Bool,
            IndexedPropertyType::Number,
            IndexedPropertyType::String,
            IndexedPropertyType::StringArray,
            IndexedPropertyType::Date
        ]);

        assert_eq!(&value, &json!([
            "bool",
            "number",
            "string",
            "string[]",
            "date"
        ]));

        assert_eq!(
            serde_json::from_value::<Vec<IndexedPropertyType>>(value).unwrap(),
            vec![
                IndexedPropertyType::Bool,
                IndexedPropertyType::Number,
                IndexedPropertyType::String,
                IndexedPropertyType::StringArray,
                IndexedPropertyType::Date
            ]
        );
    }
}
