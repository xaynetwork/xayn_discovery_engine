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

//! Code shared between various web-api* crates.
//!
//! This are mostly models and new types like e.g. `TenantId`,
//! but sometimes are also very specific utility functions like
//! a per-schema db lock.

pub mod elastic;
pub mod postgres;
pub mod request;
pub mod serde;
