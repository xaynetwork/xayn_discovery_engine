// Copyright 2022 Xayn AG
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

use std::collections::HashMap;

use serde::Deserialize;
use xayn_discovery_engine_ai::{GenericError, KeyPhrases, UserInterests};

use crate::{
    engine::{Engine, Error},
    stack::{exploration::Stack as Exploration, Data, Id},
};

const STATE_VERSION: u8 = 2;

impl Engine {
    /// Serializes the state of the engine.
    // TODO: remove the ffi for this method and reduce its visibility after DB migration
    pub async fn serialize(&self) -> Result<Vec<u8>, Error> {
        let stacks = self.stacks.read().await;
        let mut stacks_data = stacks
            .iter()
            .map(|(id, stack)| (id, &stack.data))
            .collect::<HashMap<_, _>>();
        let exploration_stack_id = Exploration::id();
        stacks_data.insert(&exploration_stack_id, &self.exploration_stack.data);
        let state = &(stacks_data, &self.user_interests, &self.key_phrases);

        // version is encoded in the first byte
        let size = 1 + bincode::serialized_size(state).map_err(Error::Serialization)?;
        #[allow(clippy::cast_possible_truncation)] // bounded by architecture
        let mut bytes = Vec::with_capacity(size as usize);
        bytes.push(STATE_VERSION);
        bincode::serialize_into(&mut bytes, state).map_err(Error::Serialization)?;

        #[cfg(feature = "storage")]
        {
            self.storage.state().store(bytes).await?;
            Ok(Vec::new())
        }

        #[cfg(not(feature = "storage"))]
        Ok(bytes)
    }

    pub(crate) fn deserialize(
        bytes: &[u8],
    ) -> Result<(HashMap<Id, Data>, UserInterests, KeyPhrases), Error> {
        match bytes.get(0) {
            Some(version) if *version < STATE_VERSION => Ok(Default::default()),
            Some(&STATE_VERSION) => {
                bincode::deserialize(&bytes[1..]).map_err(Error::Deserialization)
            }
            Some(version) => Err(GenericError::from(format!(
                "Unsupported serialized data. Found version {} expected {}",
                *version, STATE_VERSION,
            ))
            .into()),
            None => Err(GenericError::from("Empty serialized data").into()),
        }
        // Serialized data could be the partially unversioned data we had before
        .or_else(|error| {
            #[derive(Deserialize)]
            struct SerializedStackState(Vec<u8>);
            #[derive(Deserialize)]
            struct SerializedCoiSystemState(Vec<u8>);
            #[derive(Deserialize)]
            struct SerializedState {
                stacks: SerializedStackState,
                coi: SerializedCoiSystemState,
            }

            bincode::deserialize::<SerializedState>(bytes)
                .and_then(|state| {
                    bincode::deserialize::<HashMap<Id, Data>>(&state.stacks.0)
                        // deserialization might fail due to parsing error of `DateTime<Utc>` from serialized `NaiveDateTime`
                        .or_else(|_| naive_date_time_migration::deserialize(&state.stacks.0))
                        .map(|stacks| (stacks, state))
                })
                .and_then(|(stacks, state)| {
                    match state.coi.0.get(0) {
                        Some(&0) => Ok((stacks, UserInterests::default(), KeyPhrases::default())),
                        Some(&1) => {
                            #[derive(Deserialize)]
                            struct CoiSystemState {
                                user_interests: UserInterests,
                                key_phrases: KeyPhrases,
                            }

                            bincode::deserialize::<CoiSystemState>(&state.coi.0[1..])
                                .map(|coi| (stacks, coi.user_interests, coi.key_phrases))
                        }
                        // Serialized data could be the unversioned data we had before
                        _ => bincode::deserialize::<UserInterests>(bytes)
                            .map(|user_interests| (stacks, user_interests, KeyPhrases::default())),
                    }
                })
                .map_err(|_| error)
        })
    }
}

mod naive_date_time_migration {
    use crate::{
        document::{Document, Id, NewsResource, UserReaction},
        stack::{Data, Id as StackId},
    };
    use chrono::{DateTime, NaiveDateTime, Utc};
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use url::Url;
    use xayn_discovery_engine_ai::Embedding;

    #[derive(Serialize, Deserialize)]
    pub(super) struct DataWithNaiveDateTime {
        pub(super) alpha: f32,
        pub(super) beta: f32,
        pub(super) likes: f32,
        pub(super) dislikes: f32,
        pub(super) documents: Vec<DocumentWithNaiveDateTime>,
    }

    impl From<DataWithNaiveDateTime> for Data {
        fn from(data: DataWithNaiveDateTime) -> Self {
            Data {
                alpha: data.alpha,
                beta: data.beta,
                likes: data.likes,
                dislikes: data.dislikes,
                documents: data.documents.into_iter().map(Into::into).collect(),
            }
        }
    }

    #[derive(Serialize, Deserialize)]
    pub(super) struct DocumentWithNaiveDateTime {
        pub(super) id: Id,
        pub(super) stack_id: StackId,
        pub(super) smbert_embedding: Embedding,
        pub(super) reaction: Option<UserReaction>,
        pub(super) resource: NewsResourceWithNaiveDateTime,
    }

    impl From<DocumentWithNaiveDateTime> for Document {
        fn from(document: DocumentWithNaiveDateTime) -> Self {
            Document {
                id: document.id,
                stack_id: document.stack_id,
                smbert_embedding: document.smbert_embedding,
                reaction: document.reaction,
                resource: document.resource.into(),
            }
        }
    }

    #[derive(Serialize, Deserialize)]
    pub(super) struct NewsResourceWithNaiveDateTime {
        pub(super) title: String,
        pub(super) snippet: String,
        pub(super) url: Url,
        pub(super) source_domain: String,
        pub(super) date_published: NaiveDateTime,
        pub(super) image: Option<Url>,
        pub(super) rank: u64,
        pub(super) score: Option<f32>,
        pub(super) country: String,
        pub(super) language: String,
        pub(super) topic: String,
    }

    impl From<NewsResourceWithNaiveDateTime> for NewsResource {
        fn from(resource: NewsResourceWithNaiveDateTime) -> Self {
            NewsResource {
                title: resource.title,
                snippet: resource.snippet,
                url: resource.url,
                source_domain: resource.source_domain,
                date_published: DateTime::<Utc>::from_utc(resource.date_published, Utc),
                image: resource.image,
                rank: resource.rank,
                score: resource.score,
                country: resource.country,
                language: resource.language,
                topic: resource.topic,
            }
        }
    }

    pub(crate) fn deserialize(bytes: &[u8]) -> Result<HashMap<StackId, Data>, bincode::Error> {
        bincode::deserialize::<HashMap<StackId, DataWithNaiveDateTime>>(bytes).map(|stacks| {
            stacks
                .into_iter()
                .map(|(id, data)| (id, data.into()))
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::{DateTime, NaiveDate, Utc};
    use url::Url;
    use xayn_discovery_engine_ai::Embedding;

    use super::naive_date_time_migration::DataWithNaiveDateTime;
    use crate::{
        document::Id,
        stack::{Data, Id as StackId},
        state::naive_date_time_migration::{
            self,
            DocumentWithNaiveDateTime,
            NewsResourceWithNaiveDateTime,
        },
    };

    #[test]
    fn test_naive_date_time_migration() {
        let stack_id = StackId::nil();
        let date_published = NaiveDate::from_ymd(2016, 7, 8).and_hms(10, 25, 55);
        let old_document = DocumentWithNaiveDateTime {
            id: Id::new(),
            stack_id,
            reaction: None,
            smbert_embedding: Embedding::default(),
            resource: NewsResourceWithNaiveDateTime {
                title: String::default(),
                snippet: String::default(),
                url: Url::parse("https://example.net").unwrap(/* used only in tests */),
                source_domain: "example.com".to_string(),
                image: None,
                score: None,
                rank: 0,
                country: "GB".to_string(),
                language: "en".to_string(),
                topic: "news".to_string(),
                date_published,
            },
        };
        let old_data = DataWithNaiveDateTime {
            alpha: 1.,
            beta: 1.,
            likes: 1.,
            dislikes: 1.,
            documents: vec![old_document],
        };
        let old_stacks_data = HashMap::from([(stack_id, old_data)]);
        let bytes = bincode::serialize(&old_stacks_data).unwrap();
        let failing_op = bincode::deserialize::<HashMap<StackId, Data>>(&bytes);

        assert!(failing_op.is_err());

        let successful_op = naive_date_time_migration::deserialize(&bytes).unwrap();
        let new_data = successful_op.get(&stack_id).unwrap();
        let new_doc = new_data.documents.first().unwrap();
        let date_published = DateTime::<Utc>::from_utc(date_published, Utc);

        assert_eq!(new_doc.resource.date_published, date_published);
    }
}
