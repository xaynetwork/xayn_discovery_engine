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

//! Map language code to their english name.

use std::collections::HashMap;

use lazy_static::lazy_static;

lazy_static! {
    static ref CODE_TO_LANG: HashMap<&'static str, &'static str> =
        std::array::IntoIter::new([("de", "german"), ("en", "english"),]).collect();
}

pub(crate) fn get_name(code: &str) -> Option<String> {
    CODE_TO_LANG.get(code).map(|name| (*name).to_string())
}
