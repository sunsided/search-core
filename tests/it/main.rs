//! Downstream checks: each module plays one sibling crate and proves that its migration to
//! `search-core` (see `MIGRATION.md`) compiles under the real orphan rules of a separate
//! crate.

mod heuristic_search;
mod multiobjective_search;
mod stochastic_search;
