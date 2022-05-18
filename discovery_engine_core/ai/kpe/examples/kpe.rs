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

use xayn_discovery_engine_kpe::{Config, Pipeline};
use xayn_discovery_engine_test_utils::kpe::{bert, classifier, cnn, vocab};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config =
        Config::from_files(vocab()?, bert()?, cnn()?, classifier()?)?.with_token_size(128)?;

    let kpe = Pipeline::from(config)?;

    let key_phrases = kpe.run("Berlin & Brandenburg")?;
    println!("{:?}", key_phrases);
    assert_eq!(key_phrases.len(), 30);

    Ok(())
}
