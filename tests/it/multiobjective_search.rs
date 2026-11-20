//! Plays `multiobjective-search`: vector costs `[C; 2]` built from a shared scalar [`Cost`],
//! with the shared `&mut self` [`Heuristic`] in place of its `&self` one.

use search_core::{Cost, Heuristic, heuristic_fn};

/// A local zero heuristic for cost vectors, which the shared `Zero` does not cover.
struct ZeroHeuristic;

impl<S, C: Cost, const N: usize> Heuristic<S, [C; N]> for ZeroHeuristic {
    fn estimate(&mut self, _state: &S) -> Option<[C; N]> {
        Some([C::ZERO; N])
    }
}

/// Adds two cost vectors component-wise.
fn add<C: Cost, const N: usize>(a: [C; N], b: [C; N]) -> [C; N] {
    core::array::from_fn(|i| a[i] + b[i])
}

/// Sums `h(state) + edge` the way a bi-objective search computes `f`.
fn f_value<C: Cost>(
    mut heuristic: impl Heuristic<u8, [C; 2]>,
    state: u8,
    g: [C; 2],
) -> Option<[C; 2]> {
    heuristic.estimate(&state).map(|h| add(g, h))
}

/// Vector heuristics work through the local zero heuristic and through closures.
#[test]
fn vector_heuristics() {
    assert_eq!(f_value(ZeroHeuristic, 1, [2_u8, 3]), Some([2, 3]));
    assert_eq!(
        f_value(heuristic_fn(|s: &u8| Some([u32::from(*s), 1])), 4, [1, 1]),
        Some([5, 2])
    );
    assert_eq!(
        f_value(heuristic_fn(|_: &u8| None::<[u16; 2]>), 4, [1, 1]),
        None
    );
}
