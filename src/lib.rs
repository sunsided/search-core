//! # Search core
//!
//! Shared cost, heuristic, goal and budget traits for search and planning crates, with exact
//! `no_std` cost types.
//!
//! The crate holds what at least two independent search crates need, so that a heuristic,
//! goal test or budget written once works with every algorithm of the ecosystem, plus the
//! small vocabulary types of the shared [conventions](#many-agents-threads-and-frames):
//!
//! - [`Cost`]: a non-negative, totally ordered scalar path cost. Implemented for the unsigned
//!   integers (the ecosystem default is `u32`), the fixed-point [`Decimal`] with scale `10^K`,
//!   the tie-breaking pair [`Lexicographic`], and, behind the `float` feature, the checked
//!   float `NonNegF64`.
//! - [`CostExt`]: saturating subtraction and a (possibly rounding) conversion to `f64`, for
//!   priorities that subtract or divide costs; also separate from [`Cost`].
//! - [`WideMul`]: the exact product of two costs in a wider type, for comparing cost ratios
//!   by cross-multiplication. Separate from [`Cost`], so cost types without it still work.
//! - [`ExactArith`] (feature `alloc`): costs into an exact number type ([`BigUint`]) for
//!   sums of products and longer products; also separate from [`Cost`]. [`BigInt`] holds
//!   their differences, which may be negative.
//! - [`Heuristic`]: a cost-to-go estimate with a mutable workspace, plus [`heuristic_fn`] for
//!   closures and the blind heuristic [`Zero`].
//! - [`Goal`]: a goal test, implemented for closures.
//! - [`Budget`]: early stopping over a crate's own statistics type, implemented for closures,
//!   with the never-exhausted [`Unlimited`] and the [`Status`] of a time-sliced search.
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
//! ## Many agents, threads and frames
//!
//! Game AI and simulations run many searches per frame, for many entities. The planning
//! crates follow the same conventions for that:
//!
//! 1. **Shareable inputs.** Graphs, maps, games and models are `Send + Sync` and free of
//!    interior mutability, so one instance serves every thread by reference.
//! 2. **Reusable workspaces.** Everything a query mutates (open list, closed set, tables,
//!    heuristic caches) lives in an explicit workspace that is reused across queries, so a
//!    warmed-up query allocates nothing. Each thread owns its workspaces; [`Heuristic`]
//!    takes `&mut self` for this reason.
//! 3. **Time slicing.** Searches ask their [`Budget`] before each unit of work, so a limit
//!    of `n` stops after exactly `n` units, and report a [`Status`]. The counters the budget
//!    sees start from zero in every call, so a budget is the allowance of one slice. After
//!    [`Status::OutOfBudget`], a crate-specific resume call continues from the workspace;
//!    resuming a finished search should do no work. [`Unlimited`] runs to the end. Budgets are
//!    taken by value: pass `&mut closure` to keep a closure's state across slices.
//! 4. **Opt-in parallelism.** Parallel algorithms live behind a `parallel` feature; the
//!    default build stays `no_std`.
//!
//! Every type of this crate is `Send + Sync` when its type parameters are.
//!
//! ## Features
//!
//! - `alloc` (default): `Heuristic` for `Box<H>`; [`ExactArith`] with [`BigUint`] and
//!   [`BigInt`].
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
//! Repository: <https://github.com/sunsided/search-core>

#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod budget;
mod cost;
mod goal;
mod heuristic;

pub use budget::{Budget, Status, Unlimited};
#[cfg(feature = "float")]
pub use cost::NonNegF64;
#[cfg(feature = "alloc")]
pub use cost::{BigInt, BigUint, ExactArith, ExactNumber};
pub use cost::{Cost, CostExt, Decimal, Lexicographic, ParseDecimalError, WideMul};
pub use goal::Goal;
pub use heuristic::{Heuristic, HeuristicFn, Zero, heuristic_fn};

/// Doctests for the code examples in README.md (not part of the API).
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
mod readme_doctests {}
