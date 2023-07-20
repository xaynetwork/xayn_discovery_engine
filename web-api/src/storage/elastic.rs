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

mod client;
mod filter;

use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    convert::identity,
    hash::Hash,
    iter,
};

pub(crate) use client::{Client, ClientBuilder};
use either::Either;
use itertools::Itertools;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::info;
use xayn_ai_bert::NormalizedEmbedding;
pub(crate) use xayn_web_api_shared::elastic::{BulkInstruction, Config};
use xayn_web_api_shared::{
    elastic::{ScoreMap, SerdeDiscard},
    serde::{json_object, merge_json_objects, JsonObject},
};

use self::filter::Clauses;
use super::{
    property_filter::IndexedPropertiesSchemaUpdate,
    MergeFn,
    NormalizationFn,
    SearchStrategy,
};
use crate::{
    models::{
        self,
        DocumentId,
        DocumentProperties,
        DocumentProperty,
        DocumentPropertyId,
        DocumentQuery,
        DocumentSnippet,
        DocumentTags,
    },
    storage::{property_filter::IndexedPropertyType, KnnSearchParams, Warning},
    Error,
};

impl Client {
    pub(super) async fn get_by_embedding<'a>(
        &self,
        params: KnnSearchParams<'a>,
    ) -> Result<ScoreMap<DocumentId>, Error> {
        match params.strategy {
            SearchStrategy::Knn => self.knn_search(params).await,
            SearchStrategy::HybridEsRrf {
                query,
                rank_constant,
            } => {
                self.hybrid_search_es_rrf(params, query, rank_constant)
                    .await
            }
            SearchStrategy::Hybrid { query } => {
                let normalize_knn = identity;
                let normalize_bm25 = normalize_scores_if_max_gt_1;
                let merge_fn = merge_scores_average_duplicates_only;
                self.hybrid_search(params, query, normalize_knn, normalize_bm25, merge_fn)
                    .await
            }
            SearchStrategy::HybridDev {
                query,
                normalize_knn,
                normalize_bm25,
                merge_fn,
            } => {
                self.hybrid_search(
                    params,
                    query,
                    normalize_knn.to_fn(),
                    normalize_bm25.to_fn(),
                    merge_fn.to_fn(),
                )
                .await
            }
        }
    }

    async fn knn_search<'a>(
        &self,
        params: KnnSearchParams<'a>,
    ) -> Result<ScoreMap<DocumentId>, Error> {
        let KnnSearchParts {
            knn_object,
            generic_parameters,
            inner_filter: _,
        } = params.create_common_knn_search_parts();

        let request = merge_json_objects([knn_object, generic_parameters]);

        Ok(self.search_request(request).await?)
    }

    async fn hybrid_search(
        &self,
        params: KnnSearchParams<'_>,
        query: &DocumentQuery,
        normalize_knn: impl FnOnce(ScoreMap<DocumentId>) -> ScoreMap<DocumentId>,
        normalize_bm25: impl FnOnce(ScoreMap<DocumentId>) -> ScoreMap<DocumentId>,
        merge_function: impl FnOnce(ScoreMap<DocumentId>, ScoreMap<DocumentId>) -> ScoreMap<DocumentId>,
    ) -> Result<ScoreMap<DocumentId>, Error> {
        let count = params.count;

        let KnnSearchParts {
            knn_object,
            generic_parameters,
            inner_filter,
        } = params.create_common_knn_search_parts();

        let knn_request = merge_json_objects([knn_object, generic_parameters.clone()]);

        let knn_scores = self.search_request(knn_request).await?;

        let bm_25 = merge_json_objects([
            json_object!({
                "query": { "bool": merge_json_objects([
                    inner_filter,
                    json_object!({
                        "must": { "match": { "snippet": query }}
                    })
                ]) }
            }),
            generic_parameters,
        ]);
        // FIXME parallelize polling
        let bm25_scores = self.search_request(bm_25).await?;

        let merged = merge_function(normalize_knn(knn_scores), normalize_bm25(bm25_scores));
        Ok(take_highest_n_scores(count, merged))
    }

    async fn hybrid_search_es_rrf<'a>(
        &self,
        params: KnnSearchParams<'a>,
        query: &DocumentQuery,
        rank_constant: Option<u32>,
    ) -> Result<ScoreMap<DocumentId>, Error> {
        let count = params.count;

        let KnnSearchParts {
            knn_object,
            generic_parameters,
            inner_filter,
        } = params.create_common_knn_search_parts();

        let request = merge_json_objects([
            knn_object,
            generic_parameters,
            json_object!({
                "query": { "bool": merge_json_objects([
                    inner_filter,
                    json_object!({
                        "must": { "match": { "snippet": query }}
                    })
                ]) },
                "rank": {
                    "rrf": {
                        // must be >= "size"
                        "window_size": count,
                        //FIXME If we stabilize this we can omit `rank_constant` if its `None` to
                        //      safe a few bytes. But during testing we always encode it to always
                        //      run with the same parameters, even if ES changes the default.
                        "rank_constant": rank_constant.unwrap_or(60)
                    }
                }
            }),
        ]);

        Ok(self.search_request(request).await?)
    }

    pub(super) async fn insert_documents(
        &self,
        documents: &[models::IngestedDocument],
    ) -> Result<Warning<DocumentId>, Error> {
        if documents.is_empty() {
            return Ok(Warning::default());
        }

        let pre_deletions = documents
            .iter()
            .filter_map(|document| (document.embeddings.len() > 1).then_some(&document.id))
            .collect_vec();

        self.delete_by_parents(pre_deletions).await?;

        let bulk = documents
            .iter()
            .flat_map(|document: &models::IngestedDocument| {
                if document.embeddings.len() == 1 {
                    let index = BulkInstruction::Index {
                        id: document.id.as_str().to_owned(),
                    };
                    let data = EsDocument {
                        snippet: &document.snippet,
                        properties: &document.properties,
                        embedding: document.embeddings.first().unwrap(),
                        tags: &document.tags,
                        parent: None,
                    };
                    vec![
                        serde_json::to_value(index).map_err(Into::into),
                        serde_json::to_value(data).map_err(Into::into),
                    ]
                } else {
                    document
                        .embeddings
                        .iter()
                        .enumerate()
                        .flat_map(|(idx, embedding)| {
                            let index = BulkInstruction::Index {
                                id: document.id.create_child_id(idx),
                            };
                            let data = EsDocument {
                                snippet: &document.snippet,
                                properties: &document.properties,
                                embedding,
                                tags: &document.tags,
                                parent: Some(&document.id),
                            };
                            vec![
                                serde_json::to_value(index).map_err(Into::into),
                                serde_json::to_value(data).map_err(Into::into),
                            ]
                        })
                        .collect()
                }
            });

        let response = self.bulk_request(bulk).await?;
        Ok(response.failed_documents("index", false).into())
    }

    pub(super) async fn delete_by_parents(&self, parents: Vec<&DocumentId>) -> Result<(), Error> {
        let url = self.create_url(["_delete_by_query"], [("refresh", None)]);
        let body = json!({
            "query": {
                "terms": {
                    "parent": parents,
                }
            }
        });
        self.query_with_json::<_, SerdeDiscard>(Method::POST, url, Some(body))
            .await?;
        Ok(())
    }

    pub(super) async fn delete_documents(
        &self,
        ids: impl IntoIterator<Item = &DocumentDeletionHint>,
    ) -> Result<Warning<DocumentId>, Error> {
        let mut ids = ids
            .into_iter()
            .flat_map(|hint| match hint.embedding_count {
                0 => vec![],
                1 => vec![hint.id.as_str().to_owned()],
                n => (0..n).map(|idx| hint.id.create_child_id(idx)).collect_vec(),
            })
            .peekable();

        if ids.peek().is_none() {
            return Ok(Warning::default());
        }

        let response = self
            .bulk_request(ids.map(|id| Ok(BulkInstruction::Delete { id })))
            .await?;
        Ok(response.failed_documents("delete", true).into())
    }

    async fn document_property_update(
        &self,
        id: &DocumentId,
        embedding_count: usize,
        update_body: Value,
    ) -> Result<(), Error> {
        let ids = match embedding_count {
            0 => return Ok(()),
            1 => Either::Left(iter::once(id.as_str().to_owned())),
            n => Either::Right((0..n).map(|idx| id.create_child_id(idx))),
        };

        self.bulk_request::<String>(ids.flat_map(|id| {
            [
                serde_json::to_value(BulkInstruction::Update { id }),
                Ok(update_body.clone()),
            ]
        }))
        .await?
        // Hint: Allow not-found to avoid issue with race conditions if
        //       users race to delete and update and document.
        .failed_documents("update", true);

        Ok(())
    }

    pub(super) async fn insert_document_properties(
        &self,
        id: &DocumentId,
        embedding_count: usize,
        properties: &DocumentProperties,
    ) -> Result<(), Error> {
        self.document_property_update(
            id,
            embedding_count,
            json!({
                "script": {
                    "source": "ctx._source.properties = params.properties",
                    "params": {
                        "properties": properties
                    }
                },
                "_source": false
            }),
        )
        .await
    }

    pub(super) async fn delete_document_properties(
        &self,
        id: &DocumentId,
        embedding_count: usize,
    ) -> Result<(), Error> {
        self.document_property_update(
            id,
            embedding_count,
            json!({
                "script": {
                    "source": "ctx._source.properties = params.properties",
                    "params": {
                        "properties": DocumentProperties::default()
                    }
                },
                "_source": false
            }),
        )
        .await
    }

    pub(super) async fn insert_document_property(
        &self,
        document_id: &DocumentId,
        embedding_count: usize,
        property_id: &DocumentPropertyId,
        property: &DocumentProperty,
    ) -> Result<(), Error> {
        self.document_property_update(
            document_id,
            embedding_count,
            json!({
                "script": {
                    "source": "ctx._source.properties.put(params.prop_id, params.property)",
                    "params": {
                        "prop_id": property_id,
                        "property": property
                    }
                },
                "_source": false
            }),
        )
        .await
    }

    pub(super) async fn delete_document_property(
        &self,
        document_id: &DocumentId,
        embedding_count: usize,
        property_id: &DocumentPropertyId,
    ) -> Result<(), Error> {
        self.document_property_update(
            document_id,
            embedding_count,
            json!({
                "script": {
                    "source": "ctx._source.properties.remove(params.prop_id)",
                    "params": {
                        "prop_id": property_id
                    }
                },
                "_source": false
            }),
        )
        .await
    }

    pub(super) async fn insert_document_tags(
        &self,
        id: &DocumentId,
        embedding_count: usize,
        tags: &DocumentTags,
    ) -> Result<(), Error> {
        self.document_property_update(
            id,
            embedding_count,
            json!({
                "script": {
                    "source": "ctx._source.tags = params.tags",
                    "params": {
                        "tags": tags
                    }
                },
                "_source": false
            }),
        )
        .await
    }

    pub(super) async fn extend_mapping(
        &self,
        updates: &IndexedPropertiesSchemaUpdate,
        index_update_config: &IndexUpdateConfig,
    ) -> Result<(), Error> {
        if updates.len() == 0 {
            return Ok(());
        }
        let mut properties = JsonObject::with_capacity(updates.len());
        for (id, definition) in updates {
            // We ignore malformed values here for two reasons:
            // 1. set/add candidates can push documents to ES which contain
            //    malformed values
            // 2. if we somehow end up with a out-of-sync schema at least everything
            //    else but this property will still work correctly
            //
            // Note that `keyword` is excluded as it accepts anything and in turn is
            // from ES POV never malformed.
            let def = match definition.r#type {
                IndexedPropertyType::Boolean => {
                    json!({ "type": "boolean", "ignore_malformed": true })
                }
                IndexedPropertyType::Number => {
                    json!({ "type": "double", "ignore_malformed": true })
                }
                IndexedPropertyType::Keyword | IndexedPropertyType::KeywordArray => {
                    json!({ "type": "keyword" })
                }
                IndexedPropertyType::Date => json!({ "type": "date", "ignore_malformed": true }),
            };
            properties.insert(id.as_str().into(), def);
        }

        let body = json!({
            "properties": {
                "properties": {
                    // the properties of the field named properties in the properties of a document
                    "properties": properties
                }
            }
        });

        let url = self.create_url(["_mapping"], []);
        self.query_with_json::<_, SerdeDiscard>(Method::PUT, url, Some(body))
            .await?;

        info!("extended ES _mapping");

        self.update_indices(index_update_config).await?;

        Ok(())
    }

    async fn update_indices(&self, config: &IndexUpdateConfig) -> Result<(), Error> {
        let wait_for_completion = match config.method {
            IndexUpdateMethod::Background => false,
            IndexUpdateMethod::DangerWaitForCompletion => true,
        };

        let url = self.create_url(
            ["_update_by_query"],
            [
                ("conflicts", Some("proceed")),
                //FIXME add a way to async query the status of the update
                //      ES will return a task handle we can use for this.
                (
                    "wait_for_completion",
                    Some(&wait_for_completion.to_string()),
                ),
                ("refresh", Some("true")),
                (
                    "requests_per_second",
                    Some(&config.requests_per_second.to_string()),
                ),
            ],
        );
        self.query_with_json::<(), SerdeDiscard>(Method::POST, url, None)
            .await?;

        info!("started index update process");

        Ok(())
    }
}

pub(super) struct DocumentDeletionHint {
    pub(super) id: DocumentId,
    pub(super) embedding_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct IndexUpdateConfig {
    requests_per_second: usize,
    method: IndexUpdateMethod,
}

impl Default for IndexUpdateConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 500,
            method: IndexUpdateMethod::Background,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum IndexUpdateMethod {
    /// Run updates in the background without feedback for completion.
    Background,
    /// This can put the DB into an inconsistent state if it hits timeouts.
    ///
    /// Never use this in production.
    DangerWaitForCompletion,
}

#[derive(Debug, Serialize)]
pub(crate) struct EsDocument<'a> {
    pub(crate) snippet: &'a DocumentSnippet,
    pub(crate) properties: &'a DocumentProperties,
    pub(crate) embedding: &'a NormalizedEmbedding,
    pub(crate) tags: &'a DocumentTags,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) parent: Option<&'a DocumentId>,
}

struct KnnSearchParts {
    knn_object: JsonObject,
    generic_parameters: JsonObject,
    inner_filter: JsonObject,
}

impl KnnSearchParams<'_> {
    fn create_common_knn_search_parts(&self) -> KnnSearchParts {
        let Ok(Value::Object(inner_filter)) =
            serde_json::to_value(Clauses::new(self.filter, self.excluded))
        else {
            unreachable!(/* filter clauses is valid json object */);
        };
        let knn_object = self.create_knn_request_object(&inner_filter);
        let generic_parameters = json_object!({ "size": self.count });

        KnnSearchParts {
            knn_object,
            generic_parameters,
            inner_filter,
        }
    }

    fn create_knn_request_object(&self, filter: &JsonObject) -> JsonObject {
        // https://www.elastic.co/guide/en/elasticsearch/reference/current/search-search.html
        let mut obj = json_object!({
            "knn": {
                "field": "embedding",
                "query_vector": self.embedding,
                "k": self.count,
                "num_candidates": self.num_candidates,
            }
        });
        if !filter.is_empty() {
            obj["knn"]
                .as_object_mut()
                .unwrap()
                .insert("filter".into(), json!({ "bool": filter }));
        }
        obj
    }
}

fn normalize_scores<Id>(mut scores: ScoreMap<Id>) -> ScoreMap<Id> {
    let max_score = scores
        .values()
        .map(|(score, _)| score)
        .max_by(|l, r| l.total_cmp(r))
        .copied()
        .unwrap_or_default();

    for (score, _) in scores.values_mut() {
        *score /= max_score;
    }

    scores
}

fn normalize_scores_if_max_gt_1<K, M>(mut scores: HashMap<K, (f32, M)>) -> HashMap<K, (f32, M)>
where
    K: Eq + Hash,
{
    let max_score = scores
        .values()
        .map(|(score, _)| *score)
        .max_by(f32::total_cmp)
        .unwrap_or_default()
        .max(1.0);

    for (score, _) in scores.values_mut() {
        *score /= max_score;
    }

    scores
}

fn merge_scores_average_duplicates_only<Id>(
    mut scores_1: ScoreMap<Id>,
    scores_2: ScoreMap<Id>,
) -> ScoreMap<Id>
where
    Id: Hash + Eq,
{
    for (key, (score, splits)) in scores_2 {
        match scores_1.entry(key) {
            Entry::Occupied(mut oc) => {
                let entry = oc.get_mut();
                entry.0 = (entry.0 + score) / 2.;
                entry.1.extend(splits);
            }
            Entry::Vacant(vc) => {
                vc.insert((score, splits));
            }
        }
    }
    scores_1
}

fn merge_scores_weighted<Id>(scores: impl IntoIterator<Item = (f32, ScoreMap<Id>)>) -> ScoreMap<Id>
where
    Id: Hash + Eq,
{
    let weighted = scores.into_iter().flat_map(|(weight, mut scores)| {
        for (score, _) in scores.values_mut() {
            *score *= weight;
        }
        scores
    });
    collect_summing_repeated(weighted)
}

/// Reciprocal Rank Fusion
fn rrf<Id>(k: f32, scores: impl IntoIterator<Item = (f32, ScoreMap<Id>)>) -> ScoreMap<Id>
where
    Id: Hash + Eq,
{
    let rrf_scores = scores.into_iter().flat_map(|(weight, scores)| {
        scores
            .into_iter()
            .sorted_by(|(_, (s1, _)), (_, (s2, _))| s1.total_cmp(s2).reverse())
            .enumerate()
            .map(move |(rank0, (document, (_, splits)))| {
                #[allow(clippy::cast_precision_loss)]
                let rrf_score = (k + rank0 as f32 + 1.).recip() * weight;
                (document, (rrf_score, splits))
            })
    });
    collect_summing_repeated(rrf_scores)
}

fn collect_summing_repeated<Id>(
    scores: impl IntoIterator<Item = (Id, (f32, HashSet<usize>))>,
) -> ScoreMap<Id>
where
    Id: Hash + Eq,
{
    scores
        .into_iter()
        .fold(HashMap::new(), |mut acc, (key, (score, split))| {
            let (current_score, current_splits) = acc.entry(key).or_default();
            *current_score += score;
            current_splits.extend(split);
            acc
        })
}

fn take_highest_n_scores<Id>(n: usize, scores: ScoreMap<Id>) -> ScoreMap<Id>
where
    Id: Hash + Eq,
{
    if scores.len() <= n {
        return scores;
    }

    scores
        .into_iter()
        .sorted_unstable_by(|(_, (s1, _)), (_, (s2, _))| s1.total_cmp(s2).reverse())
        .take(n)
        .collect()
}

impl NormalizationFn {
    fn to_fn(self) -> Box<dyn Fn(ScoreMap<DocumentId>) -> ScoreMap<DocumentId>> {
        match self {
            NormalizationFn::Identity => Box::new(identity),
            NormalizationFn::Normalize => Box::new(normalize_scores),
            NormalizationFn::NormalizeIfMaxGt1 => Box::new(normalize_scores_if_max_gt_1),
        }
    }
}

type BoxedMergeFn = Box<dyn Fn(ScoreMap<DocumentId>, ScoreMap<DocumentId>) -> ScoreMap<DocumentId>>;

impl MergeFn {
    fn to_fn(self) -> BoxedMergeFn {
        match self {
            MergeFn::Sum {
                knn_weight,
                bm25_weight,
            } => Box::new(move |knn, bm25| {
                merge_scores_weighted([
                    (knn_weight.unwrap_or(0.5), knn),
                    (bm25_weight.unwrap_or(0.5), bm25),
                ])
            }),
            MergeFn::AverageDuplicatesOnly {} => Box::new(merge_scores_average_duplicates_only),
            MergeFn::Rrf {
                rank_constant,
                knn_weight,
                bm25_weight,
            } => {
                let rank_constant = rank_constant.unwrap_or(60.);
                let knn_weight = knn_weight.unwrap_or(1.);
                let bm25_weight = bm25_weight.unwrap_or(1.);
                Box::new(move |knn, bm25| {
                    rrf(rank_constant, [(knn_weight, knn), (bm25_weight, bm25)])
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrf_parameters_are_used() {
        let no_metadata = HashSet::default;
        let left = [
            ("foo", (2., no_metadata())),
            ("bar", (1., no_metadata())),
            ("baz", (3., no_metadata())),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();

        let right = [("baz", (5., no_metadata())), ("dodo", (1.2, no_metadata()))]
            .into_iter()
            .collect::<HashMap<_, _>>();

        assert_eq!(
            rrf(80., [(1., left.clone()), (1., right.clone())]),
            [
                ("foo", (1. / (80. + 2.), no_metadata())),
                ("bar", (1. / (80. + 3.), no_metadata())),
                ("baz", (1. / (80. + 1.) + 1. / (80. + 1.), no_metadata())),
                ("dodo", (1. / (80. + 2.), no_metadata())),
            ]
            .into_iter()
            .collect()
        );

        assert_eq!(
            rrf(80., [(0.2, left.clone()), (8., right.clone())]),
            [
                ("foo", (0.2 / (80. + 2.), no_metadata())),
                ("bar", (0.2 / (80. + 3.), no_metadata())),
                ("baz", (0.2 / (80. + 1.) + 8. / (80. + 1.), no_metadata())),
                ("dodo", (8. / (80. + 2.), no_metadata())),
            ]
            .into_iter()
            .collect()
        );
    }
}
