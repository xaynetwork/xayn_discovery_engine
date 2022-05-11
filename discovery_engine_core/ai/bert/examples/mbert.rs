// Copyright 2021 Xayn AG
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

//! Run as `cargo run --example mbert <kind>` with `<kind>`:
//! - `s` for SMBert
//! - `qa` for QAMBert

use xayn_discovery_engine_bert::{Config, FirstPooler, Pipeline, SMBert, SMBertConfig};
use xayn_discovery_engine_test_utils::smbert;

fn main() {
    let (embedding, size) = match std::env::args().nth(1).unwrap().as_str() {
        "s" => {
            let config: SMBertConfig<_> =
                Config::from_files(smbert::vocab().unwrap(), smbert::model().unwrap())
                    .unwrap()
                    .with_pooling::<FirstPooler>()
                    .with_token_size(64)
                    .unwrap();

            let mbert = Pipeline::from(config).unwrap();
            (
                mbert.run("This is a sequence.").unwrap(),
                SMBert::embedding_size(),
            )
        }
        _ => panic!("unknown MBert kind"),
    };
    println!("{}", *embedding);
    assert_eq!(embedding.shape(), [size]);
}
