use std::sync::Arc;

use derive_more::{Deref, From};
use ndarray::arr1;
#[cfg(feature = "multithreaded")]
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{
    error::Error,
};

#[derive(Clone, Deref, From)]
pub struct SMBert(Arc<rubert::SMBert>);
