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

use chrono::DateTime;
use derive_more::{Deref, Display, From, IntoIterator};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::{
    error::common::{InvalidDocumentProperty, InvalidDocumentPropertyReason},
    models::{DocumentProperty, DocumentPropertyId},
};

#[derive(Debug, Display, PartialEq, Error, Serialize)]
#[non_exhaustive]
pub(crate) enum IncompatibleUpdate {
    #[display(fmt = "Property {property_id} is already defined.")]
    PropertyIsAlreadyIndexed { property_id: DocumentPropertyId },
    #[display(
        fmt = "Only {allowed} indexed properties including publication_date are allowed, got: {count}"
    )]
    TooManyProperties { count: usize, allowed: usize },
}

//Hint: Currently the API and internal definition match so we use the same type.
#[derive(Debug, Clone, Default, Deref, IntoIterator, Serialize, Deserialize)]
pub(crate) struct IndexedPropertiesSchemaUpdate {
    #[into_iterator(owned, ref)]
    properties: HashMap<DocumentPropertyId, IndexedPropertyDefinition>,
}

//Hint: Currently the API and internal definition match so we use the same type.
#[derive(Debug, Clone, Default, Deref, IntoIterator, From, Serialize, Deserialize)]
pub(crate) struct IndexedPropertiesSchema {
    #[into_iterator(owned, ref)]
    properties: HashMap<DocumentPropertyId, IndexedPropertyDefinition>,
}

impl IndexedPropertiesSchema {
    pub(crate) fn update(
        &mut self,
        schema_update: IndexedPropertiesSchemaUpdate,
        max_total_properties: usize,
    ) -> Result<(), IncompatibleUpdate> {
        for (property_id, _) in &schema_update {
            if self.properties.contains_key(property_id) {
                return Err(IncompatibleUpdate::PropertyIsAlreadyIndexed {
                    property_id: property_id.clone(),
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
        self.properties.extend(schema_update.into_iter());
        Ok(())
    }

    pub(crate) fn validate_property(
        &self,
        property_id: &DocumentPropertyId,
        value: &DocumentProperty,
    ) -> Result<(), InvalidDocumentProperty> {
        // This code only checks additional validity constraints coming from a schema
        // but otherwise expect a valid property, hence why we take a `&DocumentProperty`
        // instead of a `&Value`.
        let value = &**value;
        let Some(definition) = self.properties.get(property_id) else {
            return Ok(());
        };
        match (value, definition.r#type) {
            (Value::Bool(_), IndexedPropertyType::Boolean)
            | (Value::Number(_), IndexedPropertyType::Number)
            | (Value::String(_), IndexedPropertyType::Keyword)
            // we only accept string arrays as valid properties
            | (Value::Array(_), IndexedPropertyType::KeywordArray) => {}
            (Value::String(string), IndexedPropertyType::Date) => {
                if DateTime::parse_from_rfc3339(string).is_err() {
                    return Err(InvalidDocumentProperty {
                        property_id: property_id.clone(),
                        invalid_value: value.clone(),
                        invalid_reason: InvalidDocumentPropertyReason::MalformedDateTimeString,
                    })
                }
            }
            (_, r#type) => {
                return Err(InvalidDocumentProperty {
                    property_id: property_id.clone(),
                    invalid_value: value.clone(),
                    invalid_reason: InvalidDocumentPropertyReason::IncompatibleType {
                        expected: r#type,
                    },
                });
            },
        }

        Ok(())
    }

    pub(crate) fn validate_filter(
        &self,
        property_id: &DocumentPropertyId,
        value: &DocumentProperty,
    ) -> Result<(), InvalidDocumentProperty> {
        // This code only checks additional validity constraints coming from a schema
        // but otherwise expect a valid property, hence why we take a `&DocumentProperty`
        // instead of a `&Value`.
        let value = &**value;
        let Some(definition) = self.properties.get(property_id) else {
            return Err(InvalidDocumentProperty {
                property_id: property_id.clone(),
                invalid_value: value.clone(),
                invalid_reason: InvalidDocumentPropertyReason::UnindexedId,
            });
        };
        match (value, definition.r#type) {
            (Value::Bool(_), IndexedPropertyType::Boolean)
            | (Value::Number(_), IndexedPropertyType::Number)
            | (Value::String(_), IndexedPropertyType::Keyword)
            // we only accept string arrays as valid properties
            | (Value::Array(_), IndexedPropertyType::Keyword | IndexedPropertyType::KeywordArray)
            => {}
            (Value::String(string), IndexedPropertyType::Date) => {
                if DateTime::parse_from_rfc3339(string).is_err() {
                    return Err(InvalidDocumentProperty {
                        property_id: property_id.clone(),
                        invalid_value: value.clone(),
                        invalid_reason: InvalidDocumentPropertyReason::MalformedDateTimeString,
                    })
                }
            }
            (_, r#type) => {
                return Err(InvalidDocumentProperty {
                    property_id: property_id.clone(),
                    invalid_value: value.clone(),
                    invalid_reason: InvalidDocumentPropertyReason::IncompatibleType {
                        expected: r#type,
                    },
                });
            },
        }

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
#[sqlx(type_name = "indexed_property_type", rename_all = "snake_case")]
pub(crate) enum IndexedPropertyType {
    Boolean,
    Number,
    Keyword,
    #[serde(rename = "keyword[]")]
    #[sqlx(rename = "keyword[]")]
    KeywordArray,
    Date,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn property_map_from_iter<'a>(
        iter: impl IntoIterator<Item = (&'a str, IndexedPropertyType)>,
    ) -> HashMap<DocumentPropertyId, IndexedPropertyDefinition> {
        iter.into_iter()
            .map(|(name, r#type)| {
                (
                    DocumentPropertyId::try_from(name).unwrap(),
                    IndexedPropertyDefinition { r#type },
                )
            })
            .collect()
    }

    impl<'a> FromIterator<(&'a str, IndexedPropertyType)> for IndexedPropertiesSchema {
        fn from_iter<T: IntoIterator<Item = (&'a str, IndexedPropertyType)>>(iter: T) -> Self {
            Self {
                properties: property_map_from_iter(iter),
            }
        }
    }

    impl<'a> FromIterator<(&'a str, IndexedPropertyType)> for IndexedPropertiesSchemaUpdate {
        fn from_iter<T: IntoIterator<Item = (&'a str, IndexedPropertyType)>>(iter: T) -> Self {
            Self {
                properties: property_map_from_iter(iter),
            }
        }
    }

    #[test]
    fn test_update_checking_works_with_no_overlap() {
        let mut schema = IndexedPropertiesSchema::from_iter([
            ("foo", IndexedPropertyType::Keyword),
            ("bar", IndexedPropertyType::Number),
        ]);
        let update = IndexedPropertiesSchemaUpdate::from_iter([
            ("foot", IndexedPropertyType::Keyword),
            ("bart", IndexedPropertyType::Number),
        ]);

        schema.update(update, 11).expect("to be compatible");
    }

    #[test]
    fn test_update_checking_works_with_overlap() {
        let mut schema = IndexedPropertiesSchema::from_iter([
            ("foo", IndexedPropertyType::Keyword),
            ("bar", IndexedPropertyType::Number),
        ]);
        let update = IndexedPropertiesSchemaUpdate::from_iter([
            ("foo", IndexedPropertyType::Keyword),
            ("bart", IndexedPropertyType::Number),
        ]);

        let err = schema.update(update, 11).unwrap_err();

        assert_eq!(
            err,
            IncompatibleUpdate::PropertyIsAlreadyIndexed {
                property_id: "foo".try_into().unwrap(),
            },
        );
    }

    #[test]
    fn test_update_checks_max_properties() {
        let mut schema =
            IndexedPropertiesSchema::from_iter([("foo", IndexedPropertyType::Keyword)]);
        let update = IndexedPropertiesSchemaUpdate::from_iter([
            ("bar", IndexedPropertyType::Keyword),
            ("baz", IndexedPropertyType::Keyword),
        ]);

        let err = schema.update(update, 2).unwrap_err();

        assert_eq!(
            err,
            IncompatibleUpdate::TooManyProperties {
                count: 3,
                allowed: 2
            }
        );

        let mut schema = IndexedPropertiesSchema::from_iter([
            ("foo", IndexedPropertyType::Keyword),
            ("bar", IndexedPropertyType::Number),
        ]);
        let update = IndexedPropertiesSchemaUpdate::from_iter([]);

        let err = schema.update(update, 1).unwrap_err();

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
            IndexedPropertyType::Boolean,
            IndexedPropertyType::Number,
            IndexedPropertyType::Keyword,
            IndexedPropertyType::KeywordArray,
            IndexedPropertyType::Date
        ]);

        assert_eq!(
            &value,
            &json!(["boolean", "number", "keyword", "keyword[]", "date"])
        );

        assert_eq!(
            serde_json::from_value::<Vec<IndexedPropertyType>>(value).unwrap(),
            vec![
                IndexedPropertyType::Boolean,
                IndexedPropertyType::Number,
                IndexedPropertyType::Keyword,
                IndexedPropertyType::KeywordArray,
                IndexedPropertyType::Date
            ]
        );
    }
}
