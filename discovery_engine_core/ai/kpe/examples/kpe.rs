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

//! Run as `cargo run --example kpe

use xayn_discovery_engine_kpe::Config;
use xayn_discovery_engine_test_utils::kpe::{bert, classifier, cnn, vocab};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kpe = Config::from_files(vocab()?, bert()?, cnn()?, classifier()?)?
        .with_token_size(128)?
        .build()?;
    let key_phrases = kpe.run("Berlin & Brandenburg")?;
    println!("{:?}", key_phrases);
    assert_eq!(key_phrases.len(), 30);

    Ok(())
}
