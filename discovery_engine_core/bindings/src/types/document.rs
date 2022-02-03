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

//! FFI functions for handling types from the document module.

#[allow(clippy::module_inception)]
mod document;
mod document_vec;
mod user_reacted;
mod user_reaction;
mod time_spent;

pub use document::*;
pub use document_vec::*;
pub use user_reacted::*;
pub use user_reaction::*;
pub use time_spent::*;
