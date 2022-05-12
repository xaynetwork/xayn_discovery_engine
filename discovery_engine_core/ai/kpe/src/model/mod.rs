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

pub(crate) mod bert;
pub(crate) mod classifier;
pub(crate) mod cnn;

use std::io::Error as IoError;

use displaydoc::Display;
use ndarray::ShapeError;
use thiserror::Error;
use tract_onnx::prelude::TractError;

use xayn_discovery_engine_layer::{conv::ConvError, io::LoadingLayerFailed};

/// The potential errors of the models.
#[derive(Debug, Display, Error)]
pub enum ModelError {
    /// Failed to read the onnx model: {0}
    Read(#[from] IoError),

    /// Failed to run a tract operation: {0}
    Tract(#[from] TractError),

    /// Invalid array shapes: {0}
    Shape(#[from] ShapeError),

    /// Failed to read or run the CNN model: {0}
    Cnn(#[from] ConvError),

    /// Failed to read the Classifier model: {0}
    Classifier(#[from] LoadingLayerFailed),

    /// Remaining parameters must be used: {0:?}
    UnusedParams(Vec<String>),

    /// The sequence must contain at least `KEY_PHRASE_SIZE` valid words
    NotEnoughWords,
}
