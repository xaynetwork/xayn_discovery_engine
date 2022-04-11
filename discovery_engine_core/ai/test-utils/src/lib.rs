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

//! The single source of truth for all data paths and other test utilities.
#![forbid(unsafe_op_in_unsafe_fn)]

mod approx_eq;
mod asset;
pub mod example;
pub mod kpe;
pub mod smbert;

pub use crate::approx_eq::ApproxEqIter;
#[doc(hidden)] // required for standalone export of assert_approx_eq!
pub use float_cmp::approx_eq;
