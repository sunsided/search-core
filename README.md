# search-core

[![crates.io](https://img.shields.io/crates/v/search-core.svg)](https://crates.io/crates/search-core)
[![docs.rs](https://img.shields.io/docsrs/search-core)](https://docs.rs/search-core)
[![CI](https://github.com/sunsided/search-core/actions/workflows/ci.yml/badge.svg)](https://github.com/sunsided/search-core/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/sunsided/search-core/graph/badge.svg)](https://codecov.io/gh/sunsided/search-core)
[![MSRV](https://img.shields.io/crates/msrv/search-core)](https://crates.io/crates/search-core)
[![License](https://img.shields.io/crates/l/search-core)](https://github.com/sunsided/search-core#license)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)

Small shared traits for planning and search crates.

A heuristic, goal test or budget written once against these traits works with every
algorithm of the ecosystem. The crate is `no_std`, has no dependencies and holds only what
at least two independent search crates need.

## Scope and status

Implemented:

- `Cost`: non-negative, totally ordered scalar path cost with `ZERO`, `saturating_add` and
  `saturating_times`. Implemented for `u8`, `u16`, `u32` (the ecosystem default), `u64`,
  `u128` and `usize`.
- `Decimal<K>`: exact fixed-point decimal cost with scale `10^K` (`K <= 19`, checked at
  compile time), stored as `u64`. `FromStr` parses `"3.25"` exactly and rejects text that
  would round (more than `K` fraction digits), signs and overflow (`ParseDecimalError`).
- `NonNegF64` (feature `float`): an `f64` cost that rejects NaN and negative values, so it
  can be `Ord`. Sound, but rounds; prefer integers or `Decimal` for exact results.
- `WideMul`: the product of two costs in a wider type (`u8`–`u64` into the next wider
  integer, `usize` into `u128`, `u128` into a `(high, low)` 256-bit pair, `Decimal<K>` into
  `u128`, `NonNegF64` into itself) and the distance `abs_diff`, exact for every integer
  and `Decimal` input. For comparing cost ratios by cross-multiplication; a separate
  trait, so `Cost` is unchanged.
- `ExactArith` (feature `alloc`): converts a cost into an exact number type
  (`ExactNumber`: `BigUint` for the integers and `Decimal`, `NonNegF64` itself) with `+`,
  `*` and a distance, for sums of products and longer products. Also separate from `Cost`.
  `NonNegF64` gains `Mul` (with `0 · ∞ = 0`).
- `Heuristic<S, C = u32>` with `&mut self` workspaces, forwarding impls for `&mut H` and
  `Box<H>` (feature `alloc`, default), `heuristic_fn` for closures, and the blind `Zero`.
- `Goal<S>`, implemented for `Fn(&S) -> bool`.
- `Budget<Stats>` over a crate's own statistics type, implemented for `FnMut(&Stats) -> bool`.

Not included, on purpose: successor generation (`Successors`, `Graph`, `Mdp`), statistics
and limits structs, and Pareto dominance. Their shapes differ between the sibling crates, or
only one crate uses them. See [MIGRATION.md](https://github.com/sunsided/search-core/blob/main/MIGRATION.md) for how each sibling would adopt
this crate.

Pre-release: version 0.0.0 reserves the name, and the API may still change before 0.1.

## Install

```sh
cargo add search-core
```

## Example

```rust
use search_core::{Cost, Decimal, Heuristic, heuristic_fn};

let mut h = heuristic_fn(|n: &u32| 10_u32.checked_sub(*n));
assert_eq!(h.estimate(&3), Some(7));

let cost = Decimal::<2>::from_raw(125) + Decimal::from_int(2).unwrap();
assert_eq!(cost.to_string(), "3.25");
assert!(Decimal::<2>::ZERO < cost);
```

## Features

- `alloc` (default): `Heuristic` for `Box<H>`, and `ExactArith` with `BigUint`.
- `float`: the checked float cost `NonNegF64`.

Without default features the crate needs neither `std` nor `alloc`.

## MSRV

Rust 1.85 (edition 2024). The minimum supported Rust version may be raised in a minor
release when a dependency or a language feature calls for it.

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
