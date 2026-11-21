//! Fuzzes [`BigUint`] arithmetic from [`ExactArith`] against the 256-bit
//! [`WideMul`] product of `u128`, which is exact and independent of `BigUint`.

#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use search_core::{BigUint, ExactArith, ExactNumber, WideMul};

/// `2^64` as a [`BigUint`], the building block for `2^128`.
fn two_pow_64() -> BigUint {
    BigUint::from(1_u128 << 64)
}

/// A 256-bit `(high, low)` pair as a [`BigUint`]: `high · 2^128 + low`.
fn from_wide((high, low): (u128, u128)) -> BigUint {
    BigUint::from(high) * two_pow_64() * two_pow_64() + BigUint::from(low)
}

/// The sum `a + b` as a 256-bit `(high, low)` pair.
fn wide_add(a: u128, b: u128) -> (u128, u128) {
    let (low, carry) = a.overflowing_add(b);

    (u128::from(carry), low)
}

fuzz_target!(|input: (u128, u128, u128, u128)| {
    let (a, b, c, d) = input;
    let left = a.exact() * b.exact();
    let right = c.exact() * d.exact();

    assert_eq!(left, from_wide(a.wide_mul(b)), "product of {a} and {b}");
    assert_eq!(
        left.cmp(&right),
        a.wide_mul(b).cmp(&c.wide_mul(d)),
        "order of {a}·{b} and {c}·{d}"
    );

    assert_eq!(
        a.exact() + b.exact(),
        from_wide(wide_add(a, b)),
        "sum of {a} and {b}"
    );
    assert_eq!(
        a.exact().abs_diff(&b.exact()),
        WideMul::abs_diff(a, b).exact(),
        "distance of {a} and {b}"
    );
});
