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
use xayn_discovery_engine_ai::{CoiConfig, CoiSystem, GenericError};
use xayn_discovery_engine_bert::{AveragePooler, SMBert, SMBertConfig};
use xayn_discovery_engine_tokenizer::{AccentChars, CaseChars};

use crate::{
    elastic,
    models::{IngestedDocument, PersonalizedDocument},
    storage::UserState,
};

pub type Db = Arc<AppState>;

#[allow(dead_code)]
pub struct AppState {
    pub(crate) smbert: SMBert,
    pub(crate) coi: CoiSystem,
    pub(crate) documents_by_id: HashMap<String, PersonalizedDocument>,
    pub(crate) documents: Vec<PersonalizedDocument>,
    pub(crate) user_state: UserState,
}

impl AppState {
    fn new(
        documents_by_id: HashMap<String, PersonalizedDocument>,
        smbert: SMBert,
        user_state: UserState,
    ) -> Self {
        let documents = documents_by_id.clone().into_values().collect();
        Self {
            documents_by_id,
            documents,
            smbert,
            coi: CoiConfig::default().build(),
            user_state,
        }
    }
}

#[derive(Clone, Debug)]
pub struct InitConfig {
    /// S-mBert vocabulary path.
    pub smbert_vocab: PathBuf,
    /// S-mBert model path.
    pub smbert_model: PathBuf,
    /// List of IngestedDocuments in JSON format.
    pub data_store: PathBuf,
    /// Handler for storing the user state.
    pub user_state: UserState,
    /// Elastic configuration.
    #[allow(dead_code)]
    pub elastic: elastic::Config,
}

// NOTE this will be removed by follow up tasks so it's not necessary to validate data here anymore
pub fn init_db(config: &InitConfig) -> Result<Db, GenericError> {
    let smbert = SMBertConfig::from_files(&config.smbert_vocab, &config.smbert_model)?
        .with_accents(AccentChars::Cleanse)
        .with_case(CaseChars::Lower)
        .with_pooling::<AveragePooler>()
        .with_token_size(64)?
        .build()?;

    let file = File::open(&config.data_store).expect("Couldn't open the data file");
    let ingestion_docs: Vec<IngestedDocument> =
        from_reader(file).expect("Couldn't deserialize json");
    let documents = ingestion_docs
        .into_iter()
        .map(|ingestion_doc| {
            let embedding = smbert.run(&ingestion_doc.snippet).unwrap();
            let document = PersonalizedDocument::new((ingestion_doc, embedding));
            (document.id.0.clone(), document)
        })
        .collect();
    let app_state = AppState::new(documents, smbert, config.user_state.clone());

    Ok(Arc::new(app_state))
}
