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

//! Ingestion service that uses Xayn Discovery Engine.

use std::{convert::Infallible, env, net::IpAddr, path::PathBuf, sync::Arc};

use envconfig::Envconfig;
use itertools::Itertools;
use serde::{de, Deserialize, Deserializer, Serialize};
use tokio::time::Instant;
use tracing::{error, info, instrument};
use tracing_subscriber::fmt::format::FmtSpan;
use warp::{
    self,
    hyper::StatusCode,
    reply::{self, Reply},
    Filter,
    Rejection,
};

use web_api::{
    get_health,
    DocumentId,
    DocumentProperties,
    DocumentProperty,
    DocumentPropertyId,
    ElasticConfig,
    ElasticDocumentData,
    ElasticState,
    Error,
};
use xayn_discovery_engine_ai::GenericError;
use xayn_discovery_engine_bert::{AvgBert, Config as BertConfig};

#[derive(Envconfig, Clone, Debug)]
pub(crate) struct Config {
    #[envconfig(from = "ELASTIC_URL", default = "http://localhost:9200")]
    pub(crate) elastic_url: String,

    #[envconfig(from = "ELASTIC_USER", default = "elastic")]
    pub(crate) elastic_user: String,

    #[envconfig(from = "ELASTIC_PASSWORD", default = "changeme")]
    pub(crate) elastic_password: String,

    #[envconfig(from = "ELASTIC_INDEX_NAME", default = "test_index")]
    pub(crate) elastic_index_name: String,

    #[envconfig(from = "PORT", default = "3030")]
    pub(crate) port: u16,

    #[envconfig(from = "IP_ADDR", default = "0.0.0.0")]
    pub(crate) ip_addr: IpAddr,

    #[envconfig(from = "BERT", default = "assets")]
    pub(crate) bert: PathBuf,

    #[envconfig(from = "MAX_BODY_SIZE", default = "524288")]
    pub(crate) max_body_size: u64,

    #[envconfig(from = "MAX_DOCUMENTS_LENGTH", default = "100")]
    pub(crate) max_documents_length: usize,
}

/// Represents a document sent for ingestion.
#[derive(Debug, Clone, Deserialize)]
struct IngestedDocument {
    /// Unique identifier of the document.
    id: DocumentId,

    /// Snippet used to calculate embeddings for a document.
    #[serde(deserialize_with = "deserialize_string_not_empty_or_zero_byte")]
    snippet: String,

    /// Contents of the document properties.
    properties: DocumentProperties,
}

#[derive(Clone, Debug, Serialize)]
struct FailedIngestionDocument {
    id: DocumentId,
}

#[derive(Debug, Clone, Serialize)]
struct IngestionError {
    /// List of Document Indices which were not successfully processed
    documents: Vec<FailedIngestionDocument>,
}

impl IngestionError {
    pub(crate) fn new(failed_documents: Vec<DocumentId>) -> Self {
        Self {
            documents: failed_documents
                .into_iter()
                .map(|id| FailedIngestionDocument { id })
                .collect(),
        }
    }

    pub(crate) fn to_reply(&self) -> impl Reply {
        reply::with_status(reply::json(self), StatusCode::INTERNAL_SERVER_ERROR)
    }
}

/// Represents body of a POST documents request.
#[derive(Debug, Clone, Deserialize)]
struct IngestionRequestBody {
    #[serde(deserialize_with = "deserialize_article_vec_not_empty")]
    documents: Vec<IngestedDocument>,
}

fn deserialize_string_not_empty_or_zero_byte<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.is_empty() {
        Err(de::Error::custom("field can't be an empty string"))
    } else if s.contains('\u{0000}') {
        Err(de::Error::custom("field can't contain zero bytes"))
    } else {
        Ok(s)
    }
}

fn deserialize_article_vec_not_empty<'de, D>(
    deserializer: D,
) -> Result<Vec<IngestedDocument>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Vec::deserialize(deserializer)?;
    if v.is_empty() {
        Err(de::Error::custom("documents can't be an empty array"))
    } else {
        Ok(v)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct DocumentPropertiesRequestBody {
    pub(crate) properties: DocumentProperties,
}

#[derive(Clone, Debug, Serialize)]
struct DocumentPropertiesResponse {
    properties: DocumentProperties,
}

impl DocumentPropertiesResponse {
    fn new(properties: DocumentProperties) -> Self {
        Self { properties }
    }

    fn to_reply(&self) -> impl Reply {
        reply::json(self)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct DocumentPropertyRequestBody {
    property: DocumentProperty,
}

#[derive(Debug, Clone, Serialize)]
struct DocumentPropertyResponse {
    property: DocumentProperty,
}

impl DocumentPropertyResponse {
    fn new(property: DocumentProperty) -> Self {
        Self { property }
    }

    fn to_reply(&self) -> impl Reply {
        reply::json(self)
    }
}

#[tokio::main]
async fn main() -> Result<(), GenericError> {
    let filter = env::var("RUST_LOG").unwrap_or_else(|_| "tracing=info,warp=debug".to_owned());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_span_events(FmtSpan::CLOSE)
        .init();

    let config = Arc::new(Config::init_from_env()?);
    let model = Arc::new(init_model(&config)?);
    let client = Arc::new(ElasticState::new(ElasticConfig {
        url: config.elastic_url.clone(),
        index_name: config.elastic_index_name.clone(),
        user: config.elastic_user.clone(),
        password: config.elastic_password.clone(),
    }));

    let routes = get_health()
        .or(post_documents(config.clone(), model, client.clone()))
        .or(get_document_properties(client.clone()))
        .or(put_document_properties(config.clone(), client.clone()))
        .or(delete_document_properties(client.clone()))
        .or(get_document_property(client.clone()))
        .or(put_document_property(config.clone(), client.clone()))
        .or(delete_document_property(client))
        .with(warp::trace::request());

    warp::serve(routes).run((config.ip_addr, config.port)).await;

    Ok(())
}

fn init_model(config: &Config) -> Result<AvgBert, GenericError> {
    info!("Bert model loading...");
    let start = Instant::now();

    let path = env::current_dir()?;
    let bert_path = path.join(&config.bert);
    let bert = BertConfig::new(&bert_path)?
        .with_pooler()
        .with_token_size(64)?
        .build()?;

    let load_duration = start.elapsed().as_secs();
    info!("Bert model loaded successfully in {} sec", load_duration);

    Ok(bert)
}

// POST /documents
fn post_documents(
    config: Arc<Config>,
    model: Arc<AvgBert>,
    client: Arc<ElasticState>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("documents"))
        .and(warp::path::end())
        .and(warp::body::content_length_limit(config.max_body_size))
        .and(warp::body::json())
        .and(with_model(model))
        .and(with_config(config))
        .and(with_client(client))
        .and_then(handle_post_documents)
        .recover(|rejection: Rejection| async {
            if let Some(error) = rejection.find::<warp::filters::body::BodyDeserializeError>() {
                error!("BodyDeserializeError: {:?}", error);
                Ok(StatusCode::BAD_REQUEST)
            } else {
                Err(rejection)
            }
        })
}

// PATH /documents/:document_id/properties
fn document_properties_path() -> impl Filter<Extract = (DocumentId,), Error = Rejection> + Clone {
    let document_id_param = warp::path::param().and_then(|document_id: String| async move {
        urlencoding::decode(&document_id)
            .map_err(Error::DocumentIdUtf8Conversion)
            .and_then(DocumentId::new)
            .map_err(warp::reject::custom)
    });

    warp::path("documents")
        .and(document_id_param)
        .and(warp::path("properties"))
}

// PATH /documents/:document_id/properties/:property_id
fn document_property_path(
) -> impl Filter<Extract = (DocumentId, DocumentPropertyId), Error = Rejection> + Clone {
    let property_id_param = warp::path::param().and_then(|property_id: String| async move {
        urlencoding::decode(&property_id)
            .map_err(Error::DocumentPropertyIdUtf8Conversion)
            .and_then(DocumentPropertyId::new)
            .map_err(warp::reject::custom)
    });

    document_properties_path().and(property_id_param)
}

// GET /documents/:document_id/properties
fn get_document_properties(
    client: Arc<ElasticState>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        .and(document_properties_path())
        .and(warp::path::end())
        .and(with_client(client))
        .and_then(handle_get_document_properties)
}

// PUT /documents/:document_id/properties
fn put_document_properties(
    config: Arc<Config>,
    client: Arc<ElasticState>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::put()
        .and(document_properties_path())
        .and(warp::path::end())
        .and(warp::body::content_length_limit(config.max_body_size))
        .and(warp::body::json())
        .and(with_client(client))
        .and_then(handle_put_document_properties)
}

// DELETE /documents/:document_id/properties
fn delete_document_properties(
    client: Arc<ElasticState>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::delete()
        .and(document_properties_path())
        .and(warp::path::end())
        .and(with_client(client))
        .and_then(handle_delete_document_properties)
}

// GET /documents/:document_id/properties/:property_id
fn get_document_property(
    client: Arc<ElasticState>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        .and(document_property_path())
        .and(warp::path::end())
        .and(with_client(client))
        .and_then(handle_get_document_property)
}

// PUT /documents/:document_id/properties/:property_id
fn put_document_property(
    config: Arc<Config>,
    client: Arc<ElasticState>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::put()
        .and(document_property_path())
        .and(warp::path::end())
        .and(warp::body::content_length_limit(config.max_body_size))
        .and(warp::body::json())
        .and(with_client(client))
        .and_then(handle_put_document_property)
}

// DELETE /documents/:document_id/properties/:property_id
fn delete_document_property(
    client: Arc<ElasticState>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::delete()
        .and(document_property_path())
        .and(warp::path::end())
        .and(with_client(client))
        .and_then(handle_delete_document_property)
}

#[instrument(skip(model, config, client))]
async fn handle_post_documents(
    body: IngestionRequestBody,
    model: Arc<AvgBert>,
    config: Arc<Config>,
    client: Arc<ElasticState>,
) -> Result<Box<dyn Reply>, Infallible> {
    if body.documents.len() > config.max_documents_length {
        error!("{} documents exceeds maximum number", body.documents.len());
        return Ok(Box::new(StatusCode::BAD_REQUEST) as Box<dyn Reply>);
    }

    let start = Instant::now();

    let (documents, failed_documents) = body
        .documents
        .into_iter()
        .map(|document| match model.run(&document.snippet) {
            Ok(embedding) => Ok((
                document.id,
                ElasticDocumentData {
                    snippet: document.snippet,
                    properties: document.properties,
                    embedding,
                },
            )),
            Err(err) => {
                error!(
                    "Document with id '{}' caused a PipelineError: {:#?}",
                    document.id, err
                );
                Err(document.id)
            }
        })
        .partition_result::<Vec<_>, Vec<_>, _, _>();

    info!(
        "{} embeddings calculated in {} sec",
        documents.len(),
        start.elapsed().as_secs(),
    );

    let response = match client.bulk_insert_documents(&documents).await {
        Ok(response) => response,
        Err(error) => {
            error!("Error storing documents: {error}");
            return Ok(Box::new(
                IngestionError::new(
                    documents
                        .into_iter()
                        .map(|(id, _)| id)
                        .chain(failed_documents)
                        .collect_vec(),
                )
                .to_reply(),
            ));
        }
    };

    let failed_documents = if response.errors {
        response
            .items
            .into_iter()
            .filter_map(|hit| {
                hit.index.error.map(|error| {
                    error!(
                        "Elastic failed to ingest document: {}; Responded with: {}, error: {:?}",
                        hit.index.id, hit.index.status, error
                    );
                    hit.index.id
                })
            })
            .chain(failed_documents)
            .collect_vec()
    } else {
        failed_documents
    };

    if failed_documents.is_empty() {
        Ok(Box::new(StatusCode::NO_CONTENT))
    } else {
        Ok(Box::new(IngestionError::new(failed_documents).to_reply()))
    }
}

#[instrument(skip(client))]
pub(crate) async fn handle_get_document_properties(
    doc_id: DocumentId,
    client: Arc<ElasticState>,
) -> Result<Box<dyn Reply>, Infallible> {
    match client.get_document_properties(&doc_id).await {
        Ok(Some(properties)) => {
            Ok(Box::new(DocumentPropertiesResponse::new(properties).to_reply()) as _)
        }
        Ok(None) => Ok(Box::new(StatusCode::NOT_FOUND) as _),
        Err(error) => {
            error!("Error fetching document properties: {error}");
            Ok(Box::new(StatusCode::BAD_REQUEST) as _)
        }
    }
}

#[instrument(skip(client))]
pub(crate) async fn handle_put_document_properties(
    doc_id: DocumentId,
    body: DocumentPropertiesRequestBody,
    client: Arc<ElasticState>,
) -> Result<StatusCode, Infallible> {
    match client
        .put_document_properties(&doc_id, &body.properties)
        .await
    {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Ok(StatusCode::NOT_FOUND),
        Err(error) => {
            error!("Error fetching document properties: {error}");
            Ok(StatusCode::BAD_REQUEST)
        }
    }
}

#[instrument(skip(client))]
pub(crate) async fn handle_delete_document_properties(
    doc_id: DocumentId,
    client: Arc<ElasticState>,
) -> Result<StatusCode, Infallible> {
    match client.delete_document_properties(&doc_id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Ok(StatusCode::NOT_FOUND),
        Err(error) => {
            error!("Error fetching document properties: {error}");
            Ok(StatusCode::BAD_REQUEST)
        }
    }
}

#[instrument(skip(client))]
pub(crate) async fn handle_get_document_property(
    doc_id: DocumentId,
    prop_id: DocumentPropertyId,
    client: Arc<ElasticState>,
) -> Result<Box<dyn Reply>, Infallible> {
    match client.get_document_property(&doc_id, &prop_id).await {
        Ok(Some(property)) => Ok(Box::new(DocumentPropertyResponse::new(property).to_reply()) as _),
        Ok(None) => Ok(Box::new(StatusCode::NOT_FOUND) as _),
        Err(error) => {
            error!("Error fetching document property: {error}");
            Ok(Box::new(StatusCode::BAD_REQUEST) as _)
        }
    }
}

#[instrument(skip(client))]
pub(crate) async fn handle_put_document_property(
    doc_id: DocumentId,
    prop_id: DocumentPropertyId,
    body: DocumentPropertyRequestBody,
    client: Arc<ElasticState>,
) -> Result<StatusCode, Infallible> {
    match client
        .put_document_property(&doc_id, &prop_id, &body.property)
        .await
    {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Ok(StatusCode::NOT_FOUND),
        Err(error) => {
            error!("Error fetching document property: {error}");
            Ok(StatusCode::BAD_REQUEST)
        }
    }
}

#[instrument(skip(client))]
pub(crate) async fn handle_delete_document_property(
    doc_id: DocumentId,
    prop_id: DocumentPropertyId,
    client: Arc<ElasticState>,
) -> Result<StatusCode, Infallible> {
    match client.delete_document_property(&doc_id, &prop_id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Ok(StatusCode::NOT_FOUND),
        Err(error) => {
            error!("Error fetching document property: {error}");
            Ok(StatusCode::BAD_REQUEST)
        }
    }
}

fn with_config(
    config: Arc<Config>,
) -> impl Filter<Extract = (Arc<Config>,), Error = Infallible> + Clone {
    warp::any().map(move || config.clone())
}

fn with_model(
    model: Arc<AvgBert>,
) -> impl Filter<Extract = (Arc<AvgBert>,), Error = Infallible> + Clone {
    warp::any().map(move || model.clone())
}

fn with_client(
    elastic: Arc<ElasticState>,
) -> impl Filter<Extract = (Arc<ElasticState>,), Error = Infallible> + Clone {
    warp::any().map(move || elastic.clone())
}
