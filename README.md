# search-core

[![crates.io](https://img.shields.io/crates/v/search-core.svg)](https://crates.io/crates/search-core)
[![docs.rs](https://img.shields.io/docsrs/search-core)](https://docs.rs/search-core)
[![CI](https://github.com/sunsided/search-core/actions/workflows/ci.yml/badge.svg)](https://github.com/sunsided/search-core/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/sunsided/search-core/graph/badge.svg)](https://codecov.io/gh/sunsided/search-core)
[![MSRV](https://img.shields.io/crates/msrv/search-core)](https://crates.io/crates/search-core)
[![License](https://img.shields.io/crates/l/search-core)](https://github.com/sunsided/search-core#license)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)

Common traits for heuristic search and planning: path costs, heuristics, goal tests and
search budgets.

Search algorithm crates use these traits as bounds, and users implement them once for their
problem: a heuristic, goal test or budget written against `search-core` works with every
algorithm that accepts it, whichever crate provides the algorithm. The crate is `no_std`,
has no dependencies, and keeps to small abstractions that are useful across many search
algorithms. Besides the plain unsigned integers, it ships the exact cost types `Decimal<K>` (fixed
point) and `Lexicographic<P, S>` (a tie-breaking pair) and a checked `f64` cost, plus the
arbitrary-precision `BigUint` and `BigInt` for exact arithmetic over costs, so that an
algorithm generic over `C: Cost` never rounds unless the caller picks a float cost.

## Who this is for

- Authors of search and planning crates who want their algorithms to accept the same
  heuristics, goal tests and budgets as the other crates of the ecosystem, instead of
  defining yet another `Heuristic` trait.
- Users of those crates who write a domain heuristic (a grid distance, a pattern database,
  a relaxed-plan estimate) or a goal test once and pass it to every algorithm that takes
  the shared trait for that cost type, such as the A\* family of heuristic-search or the
  MDP planners of stochastic-search (which use `f64` costs).
- Anyone who needs exact path costs, e.g. prices or durations with a fixed number of decimal
  places, or lexicographic costs such as "fewest steps, then fewest conflicts".
- If you are looking for an actual search algorithm rather than the traits, pick
  [heuristic-search](https://github.com/sunsided/heuristic-search) for A* and its relatives,
  [multiobjective-search](https://github.com/sunsided/multiobjective-search) for Pareto
  fronts, [mapf-search](https://github.com/sunsided/mapf-search) for multi-agent pathfinding
  or [stochastic-search](https://github.com/sunsided/stochastic-search) for MDPs and POMDPs.

## Installation

```sh
cargo add search-core
```

Or in `Cargo.toml`:

```toml
[dependencies]
search-core = "0.3"
```

## Overview

### Search interface

| Item | Purpose |
|---|---|
| `Heuristic<S, C = u32>` | Cost-to-go estimate; `None` marks a dead end. Takes `&mut self`, so a heuristic can keep a workspace. Implemented for `&mut H`, `Box<H>` (feature `alloc`) and closures via `heuristic_fn`. |
| `Zero` | The blind heuristic, for every `Cost` and for `f64`. Turns A* into Dijkstra's algorithm. |
| `Goal<S>` | Goal test, implemented for `Fn(&S) -> bool`. |
| `Budget<Stats>` | Early stopping over a search's own statistics type, implemented for `FnMut(&Stats) -> bool`. |

### Costs

| Item | Purpose |
|---|---|
| `Cost` | Non-negative, totally ordered scalar path cost with `ZERO`, `saturating_add` and `saturating_times`. Implemented for `u8` to `u128` and `usize`; `u32` is the default cost type of `Heuristic`. |
| `Decimal<K>` | Exact fixed-point cost with `K` decimal places (`K <= 19`, checked at compile time), stored as `u64`. `FromStr` parses text such as `"3.25"` exactly and rejects anything that would need rounding (`ParseDecimalError`). |
| `Lexicographic<P, S>` | A pair cost: `primary` first, `secondary` only to break ties, for example the fewest steps and then the fewest conflicts. Any search over `C: Cost` accepts it. |
| `NonNegF64` | Feature `float`: an `f64` cost that rejects NaN and negative values, so it can be `Ord`. Rounds like `f64`; prefer integers or `Decimal` for exact results. |
| `CostExt` | `saturating_sub` and a possibly rounding `to_f64`, for priorities that subtract or divide costs, as in anytime and bidirectional searches. |
| `WideMul` | Exact product of two costs in a wider type, and the distance `abs_diff`, for comparing cost ratios by cross-multiplication. |
| `ExactArith` | Feature `alloc`: converts a cost into an exact number type (`BigUint` for integers and `Decimal`) for sums of products and longer products. |
| `BigInt` | Feature `alloc`: an exact signed integer for differences of such expressions, which may be negative, such as Dinkelbach weights `den · c₁ - num · c₂`. |

`CostExt`, `WideMul` and `ExactArith` are separate from `Cost`, so a custom cost type only
implements the capabilities the algorithms it is used with ask for. Implement `Cost` for
your own type to use it with any algorithm generic over `C: Cost`.

### Out of scope

The crate defines no search algorithms and no problem representation. Successor generation
(graphs, MDPs), search statistics and limit types, and Pareto dominance differ too much
between algorithms to share, so they stay in the crates that implement the algorithms.
[MIGRATION.md](https://github.com/sunsided/search-core/blob/main/MIGRATION.md) shows how
existing search crates map their own traits onto this one.

## Example

An algorithm states the capabilities it needs as trait bounds; callers pass any
implementation:

```rust
use search_core::{Cost, Decimal, Goal, Heuristic, Zero, heuristic_fn};

/// Picks the candidate with the lowest `g + h` that is not a dead end.
fn most_promising<S, C: Cost>(candidates: &[(S, C)], h: &mut impl Heuristic<S, C>) -> Option<usize> {
    candidates
        .iter()
        .enumerate()
        .filter_map(|(i, (state, g))| h.estimate(state).map(|est| (g.saturating_add(est), i)))
        .min()
        .map(|(_, i)| i)
}

let candidates = [(4_u32, 3_u32), (7, 1), (12, 0)];

// Distance to 10; states past 10 are dead ends.
let mut to_ten = heuristic_fn(|n: &u32| 10_u32.checked_sub(*n));
assert_eq!(most_promising(&candidates, &mut to_ten), Some(1));
assert_eq!(most_promising(&candidates, &mut Zero), Some(2));

let at_ten = |n: &u32| *n == 10;
assert!(at_ten.is_goal(&10));

// Exact decimal costs, parsed without rounding.
let cost: Decimal<2> = "3.25".parse().unwrap();
assert_eq!(cost.saturating_add(Decimal::from_int(1).unwrap()).to_string(), "4.25");
assert!("3.125".parse::<Decimal<2>>().is_err());
```

## Features

| Feature | Default | Enables |
|---|---|---|
| `alloc` | yes | `Heuristic` for `Box<H>`; `ExactArith`, `ExactNumber`, `BigUint` and `BigInt` |
| `float` | no | The checked float cost `NonNegF64` |

With `default-features = false`, the crate needs neither `std` nor `alloc`.

## Minimum supported Rust version

Rust 1.85 (edition 2024). The MSRV applies to the library; tests and benchmarks may need a
newer toolchain. It may be raised in a minor release when a dependency or a language
feature calls for it.

## Contributing

See [CONTRIBUTING.md](https://github.com/sunsided/search-core/blob/main/CONTRIBUTING.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/sunsided/search-core/blob/main/LICENSE-APACHE))
- MIT license ([LICENSE-MIT](https://github.com/sunsided/search-core/blob/main/LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion
in this work by you, as defined in the Apache-2.0 license, shall be dual licensed as above,
without any additional terms or conditions.
