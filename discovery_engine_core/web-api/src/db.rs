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
use xayn_discovery_engine_ai::{CoiSystem, CoiSystemConfig, CoiSystemState};
use xayn_discovery_engine_bert::{AveragePooler, SMBert, SMBertConfig};
use xayn_discovery_engine_tokenizer::{AccentChars, CaseChars};

use crate::models::{Article, Document, UserId};

pub(crate) type Db = Arc<AppState>;

#[allow(dead_code)]
pub(crate) struct AppState {
    pub(crate) smbert: SMBert,
    pub(crate) coi: CoiSystem,
    pub(crate) documents_by_id: HashMap<String, Document>,
    pub(crate) documents: Vec<Document>,
    pub(crate) user_interests: RwLock<HashMap<UserId, CoiSystemState>>,
}

impl AppState {
    fn new(documents_by_id: HashMap<String, Document>, smbert: SMBert) -> Self {
        let documents = documents_by_id.clone().into_values().collect();
        Self {
            documents_by_id,
            documents,
            smbert,
            coi: CoiSystemConfig::default().build(),
            user_interests: RwLock::new(HashMap::new()),
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
    let smbert = SMBertConfig::from_files(&config.smbert_vocab, &config.smbert_model)?
        .with_accents(AccentChars::Cleanse)
        .with_case(CaseChars::Lower)
        .with_pooling::<AveragePooler>()
        .with_token_size(64)?
        .build()?;

    let file = File::open(&config.data_store).expect("Couldn't open the data file");
    let articles: Vec<Article> = from_reader(file).expect("Couldn't deserialize json");
    let documents = articles
        .into_iter()
        .map(|article| {
            let article_id = article
                .get("id")
                .expect("Article needs to have an 'id' field")
                .as_str()
                .expect("The article's 'id' field needs to be represented as String")
                .to_string();

            assert!(
                !article_id.trim().is_empty(),
                "The article's 'id' field can't be empty"
            );

            assert!(
                !article_id.contains('\u{0000}'),
                "The article's 'id' field can't contain zero bytes"
            );

            let description = article
                .get("description")
                .expect("Article needs to have a 'description' field")
                .as_str()
                .expect("The 'description' field needs to be represented as String");
            let embedding = smbert.run(description).unwrap();
            let document = Document::new((article, embedding));
            (article_id, document)
        })
        .collect();
    let app_state = AppState::new(documents, smbert);

    Ok(Arc::new(app_state))
}
