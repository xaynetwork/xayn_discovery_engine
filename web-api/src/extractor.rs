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
};

use derive_more::Display;
use mime::{Mime, Name};
use mime_serde_shim::Wrapper as SerDeMime;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;
use url::Url;

use crate::{
    error::common::{FileUploadNotEnabled, InvalidBinary},
    ingestion::preprocessor::PreprocessError,
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
        /// Allowed content-type. If empty everything is allowed.
        #[serde(default)]
        allowed_content_type: Vec<SerDeMime>,
    },
}

fn default_text_extractor_url() -> String {
    "http://localhost:9998".into()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            config: ExtractorConfig::Tika {
                url: default_text_extractor_url(),
                allowed_content_type: Vec::new(),
            },
        }
    }
}

#[derive(Debug, Deserialize)]
struct TikaResponse {
    #[serde(rename = "Content-Type")]
    content_type: String,

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
                allowed_content_type,
            } => {
                let url = Url::parse(url)?;
                if url.cannot_be_a_base() {
                    return Err::<_, anyhow::Error>(anyhow::anyhow!("invalid url"));
                }

                ExtractorInner::Tika {
                    client: Client::new(),
                    url,
                    allowed_content_type: allowed_content_type
                        .iter()
                        .map(|m| CmpMime(m.0.clone()))
                        .collect(),
                }
            }
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
        url: Url,
        allowed_content_type: HashSet<CmpMime>,
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
                allowed_content_type,
            } => {
                let url = url.join("/rmeta/text").unwrap(/* url is a valid base */);
                let mut response: Vec<TikaResponse> = client
                    .put(url)
                    .body(data)
                    .send()
                    .await
                    .map_err(|e| PreprocessError::Fatal(e.into()))?
                    .json()
                    // tika wasn´t able to extract the text
                    .await
                    .map_err(|e| PreprocessError::Invalid(e.into()))?;

                if response.is_empty() {
                    return Err(PreprocessError::Invalid(InvalidBinary::Unrecognized.into()));
                }
                let response = response.remove(0);

                let content_type =
                    response
                        .content_type
                        .parse()
                        .map_err(|e: mime::FromStrError| {
                            info!("Unrecognized content-type from document: {}", e.to_string());
                            PreprocessError::Invalid(InvalidBinary::Unrecognized.into())
                        })?;
                let content_type = CmpMime(content_type);

                if !allowed_content_type.is_empty() && !allowed_content_type.contains(&content_type)
                {
                    return Err(PreprocessError::Invalid(
                        InvalidBinary::ContentType {
                            found: content_type.to_string(),
                        }
                        .into(),
                    ));
                }

                if let Some(content) = response.content {
                    DocumentSnippet::new(content, usize::MAX)
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
