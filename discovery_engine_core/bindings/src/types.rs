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

//! Modules containing FFI glue for various types.

mod boxed;
pub mod date_time;
pub mod document;
pub mod duration;
pub mod embedding;
pub mod init_config;
pub mod market;
pub mod market_vec;
pub mod option;
pub mod primitives;
pub mod slice;
pub mod string;
pub mod url;
pub mod uuid;
pub mod vec;
