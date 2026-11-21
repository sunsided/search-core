//! Benchmarks for the exact cost arithmetic: ratio comparisons through
//! [`WideMul`] and sums of products through [`ExactArith`].

use std::hint::black_box;

use criterion::Criterion;
use search_core::{ExactArith, WideMul};

/// Runs every benchmark in this file with criterion's command-line flags.
fn main() {
    let mut criterion = Criterion::default().configure_from_args();

    bench_ratio(&mut criterion);
    bench_sum_of_products(&mut criterion);
    criterion.final_summary();
}

/// Compares the ratios `a / b` and `c / d` by cross-multiplication, once in
/// the next wider integer (`u64` into `u128`) and once in the 256-bit pair of
/// `u128`.
fn bench_ratio(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("ratio_less");

    group.bench_function("u64", |bencher| {
        let (a, b, c, d) = black_box((u64::MAX - 1, u64::MAX, 3, 7));
        bencher.iter(|| a.wide_mul(d) < c.wide_mul(b));
    });

    group.bench_function("u128", |bencher| {
        let (a, b, c, d) = black_box((u128::MAX - 1, u128::MAX, 3, 7));
        bencher.iter(|| a.wide_mul(d) < c.wide_mul(b));
    });

    group.finish();
}

/// Compares `(d1 · c1 + s) · c2` against `(d2 · c2 + s) · c1` in
/// [`BigUint`](search_core::BigUint), the shape that needs more than one
/// product and so cannot use [`WideMul`].
fn bench_sum_of_products(criterion: &mut Criterion) {
    let (d1, c1, d2, c2, s) = black_box((u64::MAX, u64::MAX - 3, 17_u64, u64::MAX, 7_u64));

    criterion.bench_function("sum_of_products/u64", |bencher| {
        bencher.iter(|| {
            let left = (d1.exact() * c1.exact() + s.exact()) * c2.exact();
            let right = (d2.exact() * c2.exact() + s.exact()) * c1.exact();

            left > right
        });
    });
}
