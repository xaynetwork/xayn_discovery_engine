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

use serde::Serialize;

#[derive(Serialize)]
#[serde(transparent)]
pub(crate) struct Warning {
    message: String,
}

impl From<&'_ str> for Warning {
    fn from(message: &'_ str) -> Self {
        Warning {
            message: message.into(),
        }
    }
}

impl From<String> for Warning {
    fn from(message: String) -> Self {
        Warning { message }
    }
}
