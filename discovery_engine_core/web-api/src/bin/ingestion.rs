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

use bytes::{BufMut, Bytes, BytesMut};
use chrono::{DateTime, Utc};
use envconfig::Envconfig;
use itertools::Itertools;
use reqwest::{
    header::{HeaderValue, CONTENT_TYPE},
    Client,
};
use serde::{de, Deserialize, Deserializer, Serialize};
use std::{collections::HashMap, convert::Infallible, env, path::PathBuf, sync::Arc};
use tracing::{error, info};
use tracing_subscriber::fmt::format::FmtSpan;
use warp::{self, hyper::StatusCode, reject::Reject, Filter, Rejection, Reply};
use web_api::{DocumentProperties, ElasticDocumentData};
use xayn_discovery_engine_ai::GenericError;
use xayn_discovery_engine_bert::{AveragePooler, SMBert, SMBertConfig};
use xayn_discovery_engine_tokenizer::{AccentChars, CaseChars};

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

    #[envconfig(from = "SMBERT_VOCAB", default = "assets/vocab.txt")]
    pub(crate) smbert_vocab: PathBuf,

    #[envconfig(from = "SMBERT_MODEL", default = "assets/model.onnx")]
    pub(crate) smbert_model: PathBuf,

    #[envconfig(from = "MAX_BODY_SIZE", default = "524288")]
    pub(crate) max_body_size: u64,

    #[envconfig(from = "MAX_DOCUMENTS_LENGTH", default = "100")]
    pub(crate) max_documents_length: usize,
}

/// Represents the `SMBert` model used for calculating embedding from snippets.
type Model = Arc<SMBert>;

/// Represents a document sent for ingestion.
#[derive(Debug, Clone, Deserialize)]
struct IngestedDocument {
    /// Unique identifier of the document.
    #[serde(deserialize_with = "deserialize_string_not_empty_or_zero_byte")]
    id: String,

    /// Snippet used to calculate embeddings for a document.
    #[serde(deserialize_with = "deserialize_string_not_empty_or_zero_byte")]
    snippet: String,

    /// Contents of the document properties.
    properties: DocumentProperties,
}

/// Represents an instruction for bulk insert of data into Elastic Search service.
#[derive(Debug, Serialize)]
struct BulkOpInstruction {
    index: IndexInfo,
}

impl BulkOpInstruction {
    fn new(id: String) -> BulkOpInstruction {
        BulkOpInstruction {
            index: IndexInfo { id },
        }
    }
}

#[derive(Debug, Serialize)]
struct IndexInfo {
    #[serde(rename(serialize = "_id"))]
    id: String,
}

/// Represents body of a POST documents request.
#[derive(Debug, Clone, Deserialize)]
struct IngestionRequest {
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

#[tokio::main]
async fn main() -> Result<(), GenericError> {
    let filter = env::var("RUST_LOG").unwrap_or_else(|_| "tracing=info,warp=debug".to_owned());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_span_events(FmtSpan::CLOSE)
        .init();

    let config = Config::init_from_env()?;
    let client = Client::new();
    let model = init_model(&config)?;

    let routes = post_documents(config, model, client)
        .recover(handle_rejection)
        .with(warp::trace::request());

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;

    Ok(())
}

fn init_model(config: &Config) -> Result<Model, GenericError> {
    info!("SMBert model loading...");

    let path = env::current_dir()?;
    let vocab_path = path.join(&config.smbert_vocab);
    let model_path = path.join(&config.smbert_model);
    let smbert = SMBertConfig::from_files(&vocab_path, &model_path)?
        .with_accents(AccentChars::Cleanse)
        .with_case(CaseChars::Lower)
        .with_pooling::<AveragePooler>()
        .with_token_size(64)?
        .build()?;

    info!("SMBert model loaded successfully!");

    Ok(Arc::new(smbert))
}

// POST /documents
fn post_documents(
    config: Config,
    model: Model,
    client: Client,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("documents")
        .and(warp::post())
        .and(warp::body::content_length_limit(config.max_body_size))
        .and(warp::body::json())
        .and(with_model(model))
        .and(with_config(config))
        .and(with_client(client))
        .and_then(handle_add_data)
}

async fn handle_add_data(
    body: IngestionRequest,
    model: Model,
    config: Config,
    client: Client,
) -> Result<impl warp::Reply, Rejection> {
    if body.documents.len() > config.max_documents_length {
        return Err(warp::reject::custom(TooManyDocumentsError));
    }

    let (documents, errored_ids): (Vec<_>, Vec<_>) = body
        .documents
        .into_iter()
        .map(|document| match model.run(&document.snippet) {
            Ok(embedding) => Ok((
                document.id,
                ElasticDocumentData {
                    snippet: document.snippet,
                    properties: document.properties,
                    embedding: embedding.to_vec(),
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
        .partition_result();

    if !errored_ids.is_empty() {
        return Err(warp::reject::custom(EmbeddingsCalculationError(
            errored_ids,
        )));
    }

    let bytes =
        serialize_to_ndjson(documents).map_err(|_| warp::reject::custom(SerializeNdJsonError))?;

    let url = format!(
        "{}/{}/_bulk?refresh",
        config.elastic_url, config.elastic_index_name
    );

    info!("Requesting '{}'", url);

    let _ = client
        .post(url)
        .basic_auth(&config.elastic_user, Some(&config.elastic_password))
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .body(bytes)
        .send()
        .await
        .map_err(handle_elastic_error)?
        .error_for_status()
        .map_err(handle_elastic_error)?
        .json::<HashMap<String, serde_json::Value>>()
        .await
        .map_err(|err| {
            error!("ReceivingOpError {:#?}", err);
            warp::reject::custom(ReceivingOpError)
        })?;

    Ok(StatusCode::OK)
}

fn with_config(config: Config) -> impl Filter<Extract = (Config,), Error = Infallible> + Clone {
    warp::any().map(move || config.clone())
}

fn with_model(model: Model) -> impl Filter<Extract = (Model,), Error = Infallible> + Clone {
    warp::any().map(move || model.clone())
}

fn with_client(client: Client) -> impl Filter<Extract = (Client,), Error = Infallible> + Clone {
    warp::any().map(move || client.clone())
}

fn serialize_to_ndjson(
    documents: Vec<(String, ElasticDocumentData)>,
) -> Result<Bytes, GenericError> {
    let mut bytes = BytesMut::new();

    fn write_record(
        document_id: String,
        doc_data: ElasticDocumentData,
        bytes: &mut BytesMut,
    ) -> Result<(), GenericError> {
        let bulk_op_instruction = BulkOpInstruction::new(document_id);
        let bulk_op_instruction = serde_json::to_vec(&bulk_op_instruction)?;
        let documents_bytes = serde_json::to_vec(&doc_data)?;

        bytes.put_slice(&bulk_op_instruction);
        bytes.put_u8(b'\n');
        bytes.put_slice(&documents_bytes);
        bytes.put_u8(b'\n');
        Ok(())
    }

    for (doc_id, doc_data) in documents {
        write_record(doc_id, doc_data, &mut bytes)?;
    }

    Ok(bytes.freeze())
}

#[derive(Debug)]
struct TooManyDocumentsError;
impl Reject for TooManyDocumentsError {}

#[derive(Debug)]
struct EmbeddingsCalculationError(Vec<String>);
impl Reject for EmbeddingsCalculationError {}

#[derive(Debug)]
struct SerializeNdJsonError;
impl Reject for SerializeNdJsonError {}

#[derive(Debug)]
struct ElasticOpError;
impl Reject for ElasticOpError {}

#[derive(Debug)]
struct ReceivingOpError;
impl Reject for ReceivingOpError {}

fn handle_elastic_error(err: reqwest::Error) -> Rejection {
    error!("ElasticOpError {:#?}", err);
    warp::reject::custom(ElasticOpError)
}

/// An API error serializable to JSON.
#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    errored_ids: Option<Vec<String>>,
}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let message;
    let mut errored_ids = None;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "NOT_FOUND";
    } else if err
        .find::<warp::filters::body::BodyDeserializeError>()
        .is_some()
    {
        code = StatusCode::BAD_REQUEST;
        message = "REQUEST_BODY_DESERIALIZATION_ERROR";
    } else if let Some(ElasticOpError) = err.find() {
        code = StatusCode::BAD_REQUEST;
        message = "ELASTIC_ERROR";
    } else if let Some(ReceivingOpError) = err.find() {
        code = StatusCode::BAD_REQUEST;
        message = "RECEIVING_OPERATION_ERROR";
    } else if let Some(SerializeNdJsonError) = err.find() {
        code = StatusCode::BAD_REQUEST;
        message = "NDJSON_SERIALIZATION_ERROR";
    } else if let Some(EmbeddingsCalculationError(ids)) = err.find() {
        code = StatusCode::UNPROCESSABLE_ENTITY;
        message = "UNPROCESSABLE_DOCUMENTS";
        errored_ids = Some(ids.to_vec());
    } else {
        error!("unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "UNHANDLED_REJECTION";
    }

    let json = warp::reply::json(&ErrorMessage {
        code: code.as_u16(),
        message: message.into(),
        errored_ids,
    });

    Ok(warp::reply::with_status(json, code))
}
