//! Downstream checks: each module plays one sibling crate and proves that its migration to
//! `search-core` (see `MIGRATION.md`) compiles under the real orphan rules of a separate
//! crate. `thread_safety` checks that the shared types can serve many threads.

mod heuristic_search;
mod multiobjective_search;
mod planning_qubo;
mod stochastic_search;
mod thread_safety;
