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

//! Run as `cargo run --example bert

use xayn_ai_bert::{AveragePooler, Config};
use xayn_test_utils::asset::{ort, xaynia};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pipeline = Config::new(xaynia()?, ort()?)?
        .with_token_size(250)?
        .with_pooler::<AveragePooler>()
        .build()?;
    let embedding = pipeline.run("This is a sequence.")?;
    println!("{}", *embedding);
    assert_eq!(embedding.shape(), [pipeline.embedding_size()]);

    Ok(())
}
