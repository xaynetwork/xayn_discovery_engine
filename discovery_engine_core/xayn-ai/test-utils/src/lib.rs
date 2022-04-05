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
