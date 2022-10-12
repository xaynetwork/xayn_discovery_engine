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

//! Run as `cargo run --example mbert

use xayn_discovery_engine_bert::{FirstPooler, SMBert, SMBertConfig};
use xayn_discovery_engine_test_utils::smbert;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mbert = SMBertConfig::from_files(
        smbert::vocab()?,
        #[cfg(feature = "japanese")]
        None::<&str>,
        smbert::model()?,
    )?
    .with_pooling::<FirstPooler>()
    .with_token_size(64)?
    .build()?;
    let embedding = mbert.run("This is a sequence.")?;
    let size = SMBert::embedding_size();

    println!("{}", *embedding);
    assert_eq!(embedding.shape(), [size]);

    Ok(())
}
