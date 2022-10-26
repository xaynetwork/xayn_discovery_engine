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

mod db;
mod elastic;
mod embedding;
mod error;
mod ingestion;
mod load_config;
mod logging;
mod middleware;
mod models;
mod personalization;
mod server;
mod utils;

pub use error::application::Error;
pub use ingestion::Ingestion;
pub use personalization::Personalization;
pub use server::run;
