//! # Search core
//!
//! Small shared traits for planning and search crates.
//!
//! The crate holds only what at least two independent search crates need, so that a
//! heuristic, goal test or budget written once works with every algorithm of the ecosystem:
//!
//! - [`Cost`]: a non-negative, totally ordered scalar path cost. Implemented for the unsigned
//!   integers (the ecosystem default is `u32`), the fixed-point [`Decimal`] with scale `10^K`,
//!   and, behind the `float` feature, the checked float `NonNegF64`.
//! - [`WideMul`]: the exact product of two costs in a wider type, for comparing cost ratios
//!   by cross-multiplication. Separate from [`Cost`], so cost types without it still work.
//! - [`Heuristic`]: a cost-to-go estimate with a mutable workspace, plus [`heuristic_fn`] for
//!   closures and the blind heuristic [`Zero`].
//! - [`Goal`]: a goal test, implemented for closures.
//! - [`Budget`]: early stopping over a crate's own statistics type, implemented for closures.
//!
//! There is no single "search problem" trait. Each algorithm asks for the capabilities it
//! needs, and successor generation stays in the algorithm crates, whose graph shapes differ.
//!
//! ```mermaid
//! graph LR
//!     algo[search algorithm] -->|estimate| Heuristic
//!     algo -->|is_goal| Goal
//!     algo -->|exhausted| Budget
//!     algo -->|sums, compares| Cost
//!     Heuristic -.->|"C: Cost (for Zero)"| Cost
//! ```
//!
//! ## Features
//!
//! - `alloc` (default): `Heuristic` for `Box<H>`.
//! - `float`: the `NonNegF64` cost.
//!
//! The crate is `no_std` in every configuration.
//!
//! ## Example
//!
//! ```
//! use search_core::{Budget, Cost, Goal, Heuristic, heuristic_fn};
//!
//! /// Greedy descent on a number line: a toy algorithm using all four traits.
//! fn descend<C: Cost>(
//!     mut state: u32,
//!     goal: &impl Goal<u32>,
//!     mut h: impl Heuristic<u32, C>,
//!     mut budget: impl Budget<u64>,
//! ) -> Option<u32> {
//!     let mut steps = 0_u64;
//!     while !goal.is_goal(&state) {
//!         if budget.exhausted(&steps) {
//!             return None;
//!         }
//!
//!         h.estimate(&state)?;
//!         state -= 1;
//!         steps += 1;
//!     }
//!
//!     Some(state)
//! }
//!
//! let h = heuristic_fn(|s: &u32| Some(*s));
//! assert_eq!(descend(5, &|s: &u32| *s == 2, h, |n: &u64| *n >= 10), Some(2));
//! assert_eq!(descend(5, &|s: &u32| *s == 2, h, |n: &u64| *n >= 1), None);
//! ```
//!
//! ## Status
//!
//! Pre-release; the API may still change before 0.1. See `MIGRATION.md` for how the sibling
//! crates would adopt it.
//!
//! Repository: <https://github.com/sunsided/search-core>

#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod budget;
mod cost;
mod goal;
mod heuristic;

pub use budget::Budget;
#[cfg(feature = "float")]
pub use cost::NonNegF64;
pub use cost::{Cost, Decimal, WideMul};
pub use goal::Goal;
pub use heuristic::{Heuristic, HeuristicFn, Zero, heuristic_fn};
