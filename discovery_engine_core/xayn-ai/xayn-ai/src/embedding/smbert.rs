use std::sync::Arc;

use derive_more::{Deref, From};

#[derive(Clone, Deref, From)]
pub struct SMBert(Arc<rubert::SMBert>);
