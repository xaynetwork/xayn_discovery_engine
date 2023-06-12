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

pub mod bert;
pub mod roberta;

use std::collections::{HashMap, HashSet};

use derive_more::Deref;
use ndarray::Array2;
use tokenizers::Error;
use tract_onnx::prelude::{IntoArcTensor, TValue, TVec};

use crate::config::Config;

/// The encoded sequence.
#[derive(Clone, Deref)]
pub struct Encoding {
    #[deref]
    encoding: tokenizers::Encoding,
    to_input: fn(&tokenizers::Encoding) -> TVec<TValue>,
}

impl Encoding {
    pub(crate) fn to_model_input(&self) -> TVec<TValue> {
        (self.to_input)(&self.encoding)
    }

    pub(crate) fn to_token_ids(
        &self,
        exclude: &[u32],
    ) -> Result<HashSet<i32>, <i32 as TryFrom<u32>>::Error> {
        let mut ids = HashSet::with_capacity(self.encoding.get_ids().len());
        for id in self.encoding.get_ids() {
            ids.insert(i32::try_from(*id)?);
        }
        for id in exclude {
            ids.remove(&i32::try_from(*id)?);
        }

        Ok(ids)
    }

    pub(crate) fn to_token_frequency(
        &self,
        exclude: &[u32],
    ) -> Result<HashMap<i32, usize>, <i32 as TryFrom<u32>>::Error> {
        let mut frequency = HashMap::with_capacity(self.encoding.get_ids().len());
        for id in self.encoding.get_ids() {
            frequency
                .entry(i32::try_from(*id)?)
                .and_modify(|frequency| *frequency += 1)
                .or_insert(1);
        }
        for id in exclude {
            frequency.remove(&i32::try_from(*id)?);
        }

        Ok(frequency)
    }
}

pub trait Tokenize {
    /// Creates a tokenizer from a configuration.
    fn new<P>(config: &Config<Self, P>) -> Result<Self, Error>
    where
        Self: Sized;

    /// Encodes the sequence.
    fn encode(&self, sequence: impl AsRef<str>) -> Result<Encoding, Error>;

    /// Gets the special token ids.
    fn special_token_ids(&self) -> &[u32];
}

fn tvalue_from(slice: &[u32]) -> TValue {
    TValue::Const(
        Array2::from_shape_fn((1, slice.len()), |(_, i)| i64::from(slice[i])).into_arc_tensor(),
    )
}
