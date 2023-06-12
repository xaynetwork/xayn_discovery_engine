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

use std::{fs::File, io::BufReader};

use serde::Deserialize;
use xayn_ai_bert::{tokenizer::bert::Tokenizer, Config, SparseModel};

const MODEL_DIR: &str = "change/the/dir";
const TOKEN_SIZE: usize = 160;
const SEQUENCES_FILE: &str = "change/the/file";

#[derive(Deserialize)]
struct Document {
    snippet: String,
}

fn main() {
    let config = Config::new(MODEL_DIR)
        .unwrap()
        .with_tokenizer::<Tokenizer>()
        .with_token_size(TOKEN_SIZE)
        .unwrap();
    let sequences = serde_json::from_reader::<_, Vec<Document>>(BufReader::new(
        File::open(SEQUENCES_FILE).unwrap(),
    ))
    .unwrap()
    .into_iter()
    .map(|document| document.snippet);

    SparseModel::fit(&config, sequences).unwrap();
}
