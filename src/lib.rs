//! # Search core
//!
//! Small shared traits for planning and search crates.
//!
//! ## Planned scope
//!
//! - Only traits that at least two independent search crates need
//! - Path costs, state spaces and search statistics as they become shared
//!
//! ## Status
//!
//! Pre-release. Version 0.0.0 reserves the name; the API is not started yet and will change.
//!
//! Repository: <https://github.com/sunsided/search-core>

#![no_std]

/// A path cost that can be summed and compared.
pub trait Cost: Copy + Ord + core::ops::Add<Output = Self> {
    /// The cost of the empty path.
    const ZERO: Self;
}
