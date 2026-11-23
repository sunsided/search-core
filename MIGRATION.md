# Migrating the sibling crates to search-core

How the planning/search crates map their former local traits onto `search-core`, and what
changed for their users. `tests/it/` has a stand-in module for heuristic-search (including
`NonNegF64` costs and the weighted key), stochastic-search, multiobjective-search and
planning-qubo, so those mappings are known to compile under the orphan rules of a
downstream crate.
planning-heuristics needs no stand-in: its trait is identical.

The general pattern: depend on `search-core`, delete the local trait, and re-export the
shared one from the crate root (`pub use search_core::{Goal, Heuristic, ...};`) so existing
`use my_crate::Heuristic` paths keep working.

## heuristic-search

| Local item | search-core | Change |
|---|---|---|
| `Cost` (`ZERO`, `cmp_cost`, `times`, `saturating_add`) | `Cost` | `cmp_cost` becomes `Ord::cmp`; `times` becomes `saturating_times` |
| `Cost for u32, u64, usize` | same | none |
| `Cost for i64` | dropped | **break**: signed costs void the optimality guarantees; use `u64` (only `cost.rs` uses `i64`) |
| `Cost for f64` | `NonNegF64` (feature `float`) | **break**: raw `f64` is not `Ord`; wrap edge costs with `NonNegF64::new` |
| `CostExt` (`saturating_sub`, `to_f64`) | `CostExt` | none; delete the local trait and `pub use search_core::CostExt` |
| `Heuristic<S, C>` | `Heuristic<S, C = u32>` | none (adds the `u32` default) |
| `heuristic_fn`, `HeuristicFn`, `Zero` | same | none; `Zero` covers every `Cost` |
| `Goal<S>` | `Goal<S>` | none |
| `Budget` (`exhausted(&mut self, &Stats)`) | `Budget<Stats>` | **break** in bounds: `impl Budget` becomes `impl Budget<Stats>` |
| `Limits`, `Stats`, `Weight` | stay local | `impl search_core::Budget<Stats> for Limits` |
| `Successors` | stays local | only one crate has this shape |

Code changes inside the crate: `open_list.rs` and `space.rs` replace `a.cmp_cost(&b)` with
`a.cmp(&b)`; `best_first.rs` replaces `.times(n)` with `.saturating_times(n)`. The test
`small_graphs.rs` "f64 costs work like integer costs" switches to `NonNegF64`.

## planning-heuristics

| Local item | search-core | Change |
|---|---|---|
| `Heuristic<S, C = u32>` | `Heuristic<S, C = u32>` | none, 1:1 |
| `heuristic_fn`, `HeuristicFn` | same | none, 1:1 |

`HAdd`, `HMax`, `HFf` keep their `impl Heuristic<State> for ...` blocks unchanged, now
against the re-exported trait. The crate keeps its `alloc` use, so it depends on
`search-core` with default features.

## stochastic-search

| Local item | search-core | Change |
|---|---|---|
| `Goal<S>` | `Goal<S>` | none, 1:1 |
| `Heuristic<S, C>` | `Heuristic<S, C = u32>` | none; algorithms already pin `C = f64` |
| `heuristic_fn`, `HeuristicFn` | same | none, 1:1 |
| `Zero` (`Heuristic<S, f64>`) | `Zero` | delete the local one; the shared `Zero` implements `Heuristic<S, f64>` |
| `Budget` | `Budget<Stats>` | **break** in bounds, as in heuristic-search |
| `Limits` (finite `DEFAULT`), `Stats` | stay local | `impl search_core::Budget<Stats> for Limits` |
| `Mdp` | stays local | MDP shape differs from graph successors |

With the local `Zero` gone, `pub use search_core::Zero;` keeps `stochastic_search::Zero`
working and removes the glob-import clash.

## multiobjective-search

| Local item | search-core | Change |
|---|---|---|
| `Cost` (blanket for `Copy + Ord + Add + Default`) | `Cost` | **break**: explicit impls only; user-defined cost types must implement `Cost` |
| `Default` as the zero | `Cost::ZERO`, `[C::ZERO; N]` | internal: replace `C::default()` / `W::default()` |
| `Heuristic<S, W>` with `&self` and a blanket for `Fn` | `Heuristic<S, W>` with `&mut self` | **break**: closures go through `heuristic_fn`; `boa_star(&H)` takes `H` by value (or `&mut H`) |
| `ZeroHeuristic` (`W: Default`) | stays local | implement it for `[C; N]` with `C: Cost` (see `tests/it/multiobjective_search.rs`) |
| `Dominates`, `weakly_dominates`, `add` | stay local | only one crate uses Pareto dominance |
| `Graph` | stays local | appends to a `Vec` and has no action labels, unlike `Successors` |
| goal predicate `P: Fn(&S) -> bool` | `Goal<S>` (optional) | non-breaking: every such closure already implements `Goal` |
| `fractional::Signed` (wrapping 256-bit), `fractional::big` | `BigInt`, `BigUint` (feature `alloc`) | internal: Dinkelbach weights become `BigInt::from(den.exact() * c1.exact()) - BigInt::from(num.exact() * c2.exact())`; exact without a width bound, at the cost of an allocation per weight; in hot loops use the in-place `sum += &weight` and by-reference `&a + &b` forms to avoid clones |

## planning-qubo

| Local item | search-core | Change |
|---|---|---|
| `Budget` | `Budget<Stats>` | none; already the shared trait |
| `Unlimited` | `Unlimited` | none; delete the local one and `pub use search_core::Unlimited` |
| `Status` (`Finished`, `OutOfBudget`) | `Status` | **break**: the shared enum is `#[non_exhaustive]`, so exhaustive matches outside the crate need a `_` arm |
| `Sampler` (`sample`, `resume`), `Run`, stats types | stay local | the resume call is crate-specific |

## Open questions

1. **Signed costs.** `i64` was dropped to make negative costs unrepresentable. Restore
   signed impls (with a documented non-negativity precondition) if a user needs them.
2. **`Zero` for `f64` and arrays.** `Zero` implements `Heuristic<S, f64>`. That impl
   relies on `f64` never being a `Cost`, which holds by design: raw `f64` is not `Ord`, and
   `NonNegF64` is the float cost. `[C; N]` stays local in multiobjective-search, because cost
   vectors may still become a `Cost`-like trait.
3. **Decimal storage.** `Decimal<K>` stores a `u64`. A `u32`-backed variant would halve
   open-list memory for small costs; add it (for example `Decimal<K, T = u64>`) only if a
   benchmark asks for it.
4. **Decimal parsing.** `Decimal<K>` implements `FromStr` (exact, rejecting more than `K`
   fraction digits), so pddl-rs can parse fractional `:action-costs` into it directly.
5. **`Successors`.** Kept out because only heuristic-search has the callback shape. If
   multiobjective-search adopts the callback form and a `Successors::Cost` bound that admits
   vectors, the trait can move here without breaking anything.
6. **Overflow of `+`.** `Decimal` follows `u64` (panic in debug, wrap in release) for
   consistency with the primitive costs. An always-checked `+` would trade a few cycles for
   never silently wrapping.
