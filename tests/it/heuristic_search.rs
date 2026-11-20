//! Plays `heuristic-search`: local `Stats`, `Limits` and `Successors`, a shared `Heuristic`,
//! `Goal` and `Budget`, and a small Dijkstra/A* over them, cross-checked against brute force.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

use proptest::prelude::*;
use search_core::{Budget, Cost, Decimal, Goal, Heuristic, Zero, heuristic_fn};

/// The counters of one search run, as `heuristic-search` keeps them.
#[derive(Debug, Default)]
struct Stats {
    /// States whose successors were generated.
    expanded: u64,
}

/// Counter limits, as `heuristic-search` keeps them.
struct Limits {
    /// Stop once this many states have been expanded. `None` is unbounded.
    max_expansions: Option<u64>,
}

impl Budget<Stats> for Limits {
    fn exhausted(&mut self, stats: &Stats) -> bool {
        self.max_expansions.is_some_and(|max| stats.expanded >= max)
    }
}

/// A local goal struct.
struct Target(u8);

impl Goal<u8> for Target {
    fn is_goal(&self, state: &u8) -> bool {
        *state == self.0
    }
}

/// A directed graph over `u8` states as an adjacency map with edge costs.
type Graph<C> = BTreeMap<u8, Vec<(u8, C)>>;

/// A* over `graph`; returns the cheapest goal cost, or `None` if unsolvable or out of budget.
fn astar<C: Cost>(
    graph: &Graph<C>,
    start: u8,
    goal: &impl Goal<u8>,
    mut heuristic: impl Heuristic<u8, C>,
    mut budget: impl Budget<Stats>,
) -> Option<C> {
    let mut stats = Stats::default();
    let mut best = BTreeMap::from([(start, C::ZERO)]);
    let mut open = BinaryHeap::from([Reverse((heuristic.estimate(&start)?, C::ZERO, start))]);

    while let Some(Reverse((_, g, state))) = open.pop() {
        if best.get(&state).is_some_and(|&known| known < g) {
            continue;
        }

        if goal.is_goal(&state) {
            return Some(g);
        }

        if budget.exhausted(&stats) {
            return None;
        }

        stats.expanded += 1;
        for &(next, cost) in graph.get(&state).into_iter().flatten() {
            relax(&mut best, &mut open, &mut heuristic, next, g + cost);
        }
    }

    None
}

/// The A* open list: entries `(f, g, state)`, smallest `f` first.
type Open<C> = BinaryHeap<Reverse<(C, C, u8)>>;

/// Records `g` for `next` and pushes it when it improves on the best known cost.
fn relax<C: Cost>(
    best: &mut BTreeMap<u8, C>,
    open: &mut Open<C>,
    heuristic: &mut impl Heuristic<u8, C>,
    next: u8,
    g: C,
) {
    if best.get(&next).is_some_and(|&known| known <= g) {
        return;
    }

    best.insert(next, g);
    if let Some(h) = heuristic.estimate(&next) {
        open.push(Reverse((g.saturating_add(h), g, next)));
    }
}

/// The cheapest cost from `start` to `goal` by Bellman-Ford relaxation, the reference.
fn reference(graph: &Graph<u32>, start: u8, goal: u8) -> Option<u32> {
    let mut dist = BTreeMap::from([(start, 0_u32)]);
    for _ in 0..=graph.len() {
        for (&from, edges) in graph {
            let Some(&d) = dist.get(&from) else { continue };
            for &(to, cost) in edges {
                let entry = dist.entry(to).or_insert(u32::MAX);
                *entry = (*entry).min(d + cost);
            }
        }
    }

    dist.get(&goal).copied()
}

/// A diamond `0 -> {1, 2} -> 3` where the route through `2` is cheaper.
fn diamond() -> Graph<u32> {
    BTreeMap::from([
        (0, vec![(1, 1), (2, 2)]),
        (1, vec![(3, 5)]),
        (2, vec![(3, 1)]),
    ])
}

/// `Zero`, closures and `&mut h` all work as heuristics.
#[test]
fn heuristic_forms_compile_and_agree() {
    let graph = diamond();
    let unlimited = || Limits {
        max_expansions: None,
    };
    let mut table = heuristic_fn(|s: &u8| Some(u32::from(*s == 0)));

    assert_eq!(astar(&graph, 0, &Target(3), Zero, unlimited()), Some(3));
    assert_eq!(
        astar(&graph, 0, &Target(3), &mut table, unlimited()),
        Some(3)
    );
    assert_eq!(table.estimate(&0), Some(1));
}

/// `Box<dyn Heuristic>` works as a heuristic (feature `alloc`).
#[cfg(feature = "alloc")]
#[test]
fn boxed_dyn_heuristic() {
    let boxed: Box<dyn Heuristic<u8>> = Box::new(heuristic_fn(|_: &u8| Some(0)));

    assert_eq!(
        astar(&diamond(), 0, &Target(3), boxed, |_: &Stats| false),
        Some(3)
    );
}

/// Closure goals and closure budgets work alongside the local structs.
#[test]
fn closure_goal_and_budget() {
    let graph = diamond();

    assert_eq!(
        astar(&graph, 0, &|s: &u8| *s == 3, Zero, |_: &Stats| false),
        Some(3)
    );
    assert_eq!(
        astar(
            &graph,
            0,
            &Target(3),
            Zero,
            Limits {
                max_expansions: Some(1)
            }
        ),
        None
    );
    assert_eq!(
        astar(&graph, 0, &Target(3), Zero, |s: &Stats| s.expanded >= 3),
        Some(3)
    );
}

/// The same search runs on fixed-point costs.
#[test]
fn decimal_costs() {
    let cost = Decimal::<2>::from_raw;
    let graph: Graph<Decimal<2>> = BTreeMap::from([
        (0, vec![(1, cost(150)), (2, cost(75))]),
        (2, vec![(1, cost(50))]),
    ]);

    let found = astar(&graph, 0, &Target(1), Zero, |_: &Stats| false);

    assert_eq!(found, Some(cost(125)));
    assert_eq!(found.unwrap().to_string(), "1.25");
}

/// The search runs on checked float costs (feature `float`).
#[cfg(feature = "float")]
#[test]
fn float_costs() {
    let cost = |value| search_core::NonNegF64::new(value).expect("valid cost");
    let graph = BTreeMap::from([
        (0, vec![(1, cost(2.5)), (2, cost(0.5))]),
        (2, vec![(1, cost(1.0))]),
    ]);

    let found = astar(&graph, 0, &Target(1), Zero, |_: &Stats| false);

    assert_eq!(found.map(search_core::NonNegF64::get), Some(1.5));
}

/// The weighted A* key `den * g + num * h` saturates instead of overflowing.
#[test]
fn weighted_key_saturates() {
    /// Weighted A* key with weight `num / den`.
    fn key<C: Cost>(g: C, h: C, num: u32, den: u32) -> C {
        g.saturating_times(den)
            .saturating_add(h.saturating_times(num))
    }

    assert_eq!(key(4_u32, 3, 3, 2), 17);
    assert_eq!(key(u32::MAX / 2, 1, 1, 3), u32::MAX);
    assert_eq!(
        key(Decimal::<1>::from_raw(5), Decimal::from_raw(5), 2, 1),
        Decimal::from_raw(15)
    );
}

proptest! {
    /// A* with the blind heuristic finds the same cost as Bellman-Ford on random graphs.
    #[test]
    fn astar_matches_reference(
        edges in prop::collection::vec((0_u8..8, 0_u8..8, 0_u32..20), 0..30),
        goal in 0_u8..8,
    ) {
        let mut graph = Graph::new();
        for (from, to, cost) in edges {
            graph.entry(from).or_insert_with(Vec::new).push((to, cost));
        }

        let found = astar(&graph, 0, &Target(goal), Zero, |_: &Stats| false);

        prop_assert_eq!(found, reference(&graph, 0, goal));
    }
}
