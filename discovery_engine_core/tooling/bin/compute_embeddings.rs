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

use anyhow::Result;
use serde_json::{json, Value};
use std::{env, io, io::Write, process::exit};
use xayn_discovery_engine_ai::{Builder, Embedding, Ranker};
use xayn_discovery_engine_bert::{AveragePooler, SMBertConfig};
use xayn_discovery_engine_core::{ai_config_from_json, InitConfig};
use xayn_discovery_engine_kpe::Config as KpeConfig;
use xayn_discovery_engine_tokenizer::{AccentChars, CaseChars};

fn embedding_to_json_value(embedding: Embedding) -> Value {
    let embedding_values = &(*embedding);
    let inner: Vec<_> = embedding_values
        .iter()
        .map(|value| Value::from(*value))
        .collect();
    Value::Array(inner)
}

fn init_ranker() -> Result<Ranker> {
    let ai_config = ai_config_from_json("{}");
    let asset_base = "../../discovery_engine_flutter/example/assets/";
    let config = InitConfig {
        api_key: "".to_string(),
        api_base_url: "".to_string(),
        markets: vec![],
        trusted_sources: vec![],
        excluded_sources: vec![],
        smbert_vocab: format!("{}/smbert_v0001/vocab.txt", asset_base),
        smbert_model: format!("{}/smbert_v0001/smbert-quantized.onnx", asset_base),
        kpe_vocab: format!("{}/kpe_v0001/vocab.txt", asset_base),
        kpe_model: format!("{}/kpe_v0001/bert-quantized.onnx", asset_base),
        kpe_cnn: format!("{}/kpe_v0001/cnn.binparams", asset_base),
        kpe_classifier: format!("{}/kpe_v0001/classifier.binparams", asset_base),
        ai_config: None,
    };

    let smbert_config = SMBertConfig::from_files(&config.smbert_vocab, &config.smbert_model)?
        .with_token_size(150)?
        .with_accents(AccentChars::Cleanse)
        .with_case(CaseChars::Lower)
        .with_pooling::<AveragePooler>();

    let kpe_config = KpeConfig::from_files(
        &config.kpe_vocab,
        &config.kpe_model,
        &config.kpe_cnn,
        &config.kpe_classifier,
    )?
    .with_token_size(150)?
    .with_accents(AccentChars::Cleanse)
    .with_case(CaseChars::Keep);

    let coi_system_config = ai_config.extract()?;

    let builder =
        Builder::from(smbert_config, kpe_config).with_coi_system_config(coi_system_config);

    Ok(builder.build().unwrap())
}

fn main() -> Result<()> {
    let args: Vec<_> = env::args().collect();
    if args.len() > 1 && args[1] == "-h" {
        eprintln!(
            r#"
usage:

    cat input.json | compute_embeddings > output.json

        Takes an input ndjson (newline-delimited json) file, computes
        the embeddings for each entry (line), based on the entries'
        excerpt. Writes the result to stdout.

    compute_embeddings [string]

        Computes the embedding for the given string
"#
        );
        exit(1);
    }

    let ranker = init_ranker().unwrap();
    if args.len() > 1 {
        let input = &args[1];
        let embedding = ranker.compute_smbert(input).unwrap();
        let json = json!({
            "embedding": embedding_to_json_value(embedding),
        });
        writeln!(io::stdout(), "{}", serde_json::to_string(&json)?)?;
        return Ok(());
    }

    let mut line_buffer = String::new();
    loop {
        line_buffer.clear();
        match io::stdin().read_line(&mut line_buffer) {
            Ok(_) => {
                let trimmed = line_buffer.trim();
                if trimmed.is_empty() {
                    break;
                }

                let mut json: Value = serde_json::from_str(line_buffer.trim()).unwrap();
                let excerpt = &json["excerpt"].as_str().unwrap();
                let embedding = ranker.compute_smbert(excerpt).unwrap();
                json["embedding"] = embedding_to_json_value(embedding);
                writeln!(io::stdout(), "{}", serde_json::to_string(&json)?)?;
                eprint!(".");
            }
            Err(err) => {
                eprintln!("{}", err);
                break;
            }
        }
    }

    Ok(())
}
