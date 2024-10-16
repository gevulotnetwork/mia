//! MIA runtime config.

#[cfg(feature = "v1")]
#[path = "v1/mod.rs"]
mod implementation;

pub use implementation::*;
