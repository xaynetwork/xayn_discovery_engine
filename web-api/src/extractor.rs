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

use std::{
    cmp::Ordering,
    collections::HashSet,
    hash::{Hash, Hasher},
    time::Duration,
};

use derive_more::Display;
use mime::{Mime, Name};
use mime_serde_shim::Wrapper as SerDeMime;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use tracing::info;
use url::Url;
use xayn_web_api_shared::{elastic::SegmentableUrl, serde::serde_duration_in_config};

use crate::{
    backoffice::preprocessor::PreprocessError,
    error::common::{FileUploadNotEnabled, InvalidBinary},
    models::DocumentSnippet,
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    /// File text extraction is available.
    #[serde(default)]
    pub(crate) enabled: bool,
    #[serde(flatten)]
    config: ExtractorConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "extractor")]
pub enum ExtractorConfig {
    #[serde(rename = "tika")]
    Tika {
        /// Tika server to contact
        #[serde(default = "default_text_extractor_url")]
        url: String,
        /// Allowed media type. If empty allows everything.
        #[serde(default)]
        allowed_media_type: Vec<SerDeMime>,
        /// Request timeout in milliseconds.
        /// If Tika takes more than this to extract the text the document is too complex
        /// and we mark the document as invalid.
        #[serde(with = "serde_duration_in_config", default = "default_timeout")]
        timeout: Duration,
    },
}

fn default_text_extractor_url() -> String {
    "http://localhost:9998".into()
}

fn default_timeout() -> Duration {
    Duration::from_secs(5)
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            config: ExtractorConfig::Tika {
                url: default_text_extractor_url(),
                allowed_media_type: Vec::new(),
                timeout: default_timeout(),
            },
        }
    }
}

#[derive(Debug, Deserialize)]
struct TikaResponse {
    #[serde(rename = "Content-Type")]
    media_type: String,

    #[serde(rename = "X-TIKA:content")]
    content: Option<String>,
}

// We want to compare only the type and subtype
#[derive(Debug, Display)]
struct CmpMime(Mime);

impl CmpMime {
    fn project(&self) -> (Name<'_>, Name<'_>) {
        (self.0.type_(), self.0.subtype())
    }
}

impl PartialEq for CmpMime {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}
impl Eq for CmpMime {}

impl PartialOrd for CmpMime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CmpMime {
    fn cmp(&self, other: &Self) -> Ordering {
        self.project().cmp(&other.project())
    }
}

impl Hash for CmpMime {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.project().hash(state);
    }
}

/// Extract text and other information from different format
pub(crate) struct TextExtractor {
    inner: ExtractorInner,
}

impl TextExtractor {
    pub(crate) fn new(config: &Config) -> Result<Self, anyhow::Error> {
        if !config.enabled {
            return Ok(Self {
                inner: ExtractorInner::Disabled,
            });
        }

        let inner = match &config.config {
            ExtractorConfig::Tika {
                url,
                allowed_media_type,
                timeout,
            } => ExtractorInner::Tika {
                client: ClientBuilder::new().timeout(*timeout).build()?,
                url: url.parse()?,
                allowed_media_type: allowed_media_type
                    .iter()
                    .map(|m| CmpMime(m.0.clone()))
                    .collect(),
            },
        };

        Ok(Self { inner })
    }

    #[allow(clippy::unused_async)]
    pub(crate) async fn extract_text(
        &self,
        data: Vec<u8>,
    ) -> Result<DocumentSnippet, PreprocessError> {
        self.inner.extract_text(data).await
    }
}

enum ExtractorInner {
    Disabled,
    Tika {
        client: Client,
        url: SegmentableUrl,
        allowed_media_type: HashSet<CmpMime>,
    },
}

impl ExtractorInner {
    pub(crate) async fn extract_text(
        &self,
        data: Vec<u8>,
    ) -> Result<DocumentSnippet, PreprocessError> {
        if data.is_empty() {
            return Err(PreprocessError::Invalid(
                InvalidBinary::InvalidContent.into(),
            ));
        }

        match self {
            Self::Disabled => Err(PreprocessError::Invalid(FileUploadNotEnabled.into())),
            Self::Tika {
                client,
                url,
                allowed_media_type,
            } => {
                let url: Url = url.with_segments(["rmeta", "text"]).into();
                let mut response: Vec<TikaResponse> = client
                    .put(url)
                    .body(data)
                    .send()
                    .await
                    .map_err(|e| {
                        if e.is_timeout() {
                            PreprocessError::Invalid(InvalidBinary::ContentTooComplex.into())
                        } else {
                            PreprocessError::Fatal(e.into())
                        }
                    })?
                    .json()
                    // tika wasn´t able to extract the text
                    .await
                    .map_err(|e| PreprocessError::Invalid(e.into()))?;

                if response.is_empty() {
                    return Err(PreprocessError::Invalid(InvalidBinary::Unrecognized.into()));
                }
                let response = response.remove(0);

                let media_type = response
                    .media_type
                    .parse()
                    .map_err(|e: mime::FromStrError| {
                        info!("Unrecognized media type: {}", e.to_string());
                        PreprocessError::Invalid(InvalidBinary::Unrecognized.into())
                    })?;
                let content_type = CmpMime(media_type);

                if !allowed_media_type.is_empty() && !allowed_media_type.contains(&content_type) {
                    return Err(PreprocessError::Invalid(
                        InvalidBinary::MediaType {
                            found: content_type.to_string(),
                        }
                        .into(),
                    ));
                }

                if let Some(content) = response.content {
                    DocumentSnippet::new_with_length_constraint(content, 1..)
                        .map_err(|e| PreprocessError::Invalid(e.into()))
                } else {
                    Err(PreprocessError::Invalid(
                        InvalidBinary::InvalidContent.into(),
                    ))
                }
            }
        }
    }
}
