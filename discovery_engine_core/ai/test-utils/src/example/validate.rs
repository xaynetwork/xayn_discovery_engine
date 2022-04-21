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

use std::{io::Result, path::PathBuf};

use crate::asset::{resolve_path, DATA_DIR};

const ASSET: &str = "ted_talk_transcripts.csv";

/// Resolves the path to the `MBert` validation transcripts.
pub fn transcripts() -> Result<PathBuf> {
    resolve_path(&[DATA_DIR, ASSET])
}
