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

use std::cell::RefCell;

use crate::{
    reranker::database::{Database, RerankerData},
    Error,
};

pub(crate) struct MemDb {
    data: RefCell<Option<RerankerData>>,
}

impl MemDb {
    pub(crate) fn new() -> Self {
        Self {
            data: RefCell::new(None),
        }
    }

    pub(crate) fn from_data(data: RerankerData) -> Self {
        Self {
            data: RefCell::new(data.into()),
        }
    }
}

impl Database for MemDb {
    fn serialize(&self, _data: &RerankerData) -> Result<Vec<u8>, Error> {
        unimplemented!("mocked database does not have a serialized representation")
    }

    fn load_data(&self) -> Result<Option<RerankerData>, Error> {
        Ok(self.data.borrow().clone())
    }
}
