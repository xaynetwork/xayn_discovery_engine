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

use serde_json::from_reader;
use std::{collections::HashMap, fs::File, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use xayn_discovery_engine_ai::UserInterests;
use xayn_discovery_engine_bert::{AveragePooler, SMBertConfig};
use xayn_discovery_engine_tokenizer::{AccentChars, CaseChars};

use crate::models::{Article, Document, Id, UserId};

pub(crate) type Db = Arc<RwLock<AppState>>;

#[allow(dead_code)]
pub(crate) struct AppState {
    pub(crate) documents: HashMap<Id, Document>,
    pub(crate) user_interests: HashMap<UserId, UserInterests>,
}

impl AppState {
    fn new(documents: HashMap<Id, Document>) -> Self {
        Self {
            documents,
            user_interests: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct InitConfig {
    /// S-mBert vocabulary path.
    pub(crate) smbert_vocab: PathBuf,
    /// S-mBert model path.
    pub(crate) smbert_model: PathBuf,
    /// List of [Article]s in JSON format.
    pub(crate) data_store: PathBuf,
}

pub(crate) fn init_db(config: &InitConfig) -> Result<Db, Box<dyn std::error::Error>> {
    let file = File::open(&config.data_store).expect("Couldn't open the file");
    let mbert = SMBertConfig::from_files(&config.smbert_vocab, &config.smbert_model)?
        .with_accents(AccentChars::Cleanse)
        .with_case(CaseChars::Lower)
        .with_pooling::<AveragePooler>()
        .with_token_size(64)?
        .build()?;
    let articles: Vec<Article> = from_reader(file).expect("Couldn't deserialize json");
    let documents = articles
        .into_iter()
        .map(|article| {
            let embedding = mbert.run(&article.description).unwrap();
            let document = Document::new((article, embedding));

            (document.id, document)
        })
        .collect();
    let app_state = AppState::new(documents);

    Ok(Arc::new(RwLock::new(app_state)))
}
