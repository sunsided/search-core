# search-core

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
  compile time), stored as `u64`.
- `NonNegF64` (feature `float`): an `f64` cost that rejects NaN and negative values, so it
  can be `Ord`. Sound, but rounds; prefer integers or `Decimal` for exact results.
- `Heuristic<S, C = u32>` with `&mut self` workspaces, forwarding impls for `&mut H` and
  `Box<H>` (feature `alloc`, default), `heuristic_fn` for closures, and the blind `Zero`.
- `Goal<S>`, implemented for `Fn(&S) -> bool`.
- `Budget<Stats>` over a crate's own statistics type, implemented for `FnMut(&Stats) -> bool`.

Not included, on purpose: successor generation (`Successors`, `Graph`, `Mdp`), statistics
and limits structs, and Pareto dominance. Their shapes differ between the sibling crates, or
only one crate uses them. See [MIGRATION.md](MIGRATION.md) for how each sibling would adopt
this crate.

Pre-release: version 0.0.0 reserves the name, and the API may still change before 0.1.

## Example

```rust
use search_core::{Cost, Decimal, Heuristic, heuristic_fn};

let mut h = heuristic_fn(|n: &u32| 10_u32.checked_sub(*n));
assert_eq!(h.estimate(&3), Some(7));

let cost = Decimal::<2>::from_raw(125) + Decimal::from_int(2).unwrap();
assert_eq!(cost.to_string(), "3.25");
assert!(Decimal::<2>::ZERO < cost);
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
