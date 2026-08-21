//! Simple-path enumeration for multihop route discovery (top-K by hop count).
//!
//! Soundness ([GitLab #286](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/286)):
//! the enumerating DFS never blindly truncates. A single BFS from the goal precomputes each node's
//! minimum hop-distance to the goal, and the DFS only descends into a neighbour that can still
//! reach the goal within the remaining hop budget. This pruning is **admissible** — it can never
//! drop a valid route, because any real continuation must satisfy the hop budget — yet it collapses
//! the adversarial case from O(branching^max_hops) to O(V+E): a dense graph with an unreachable
//! goal (the abuse case in #286) leaves the goal absent from the BFS, so the DFS never starts. The
//! same reachability gate covers the early-exit ask (an input/output token with no pairs is simply
//! unreachable). Selection is by iterative deepening, so the `max_paths` cap always keeps the
//! fewest-hop routes (the natural execution priority) rather than an arbitrary subset — a route is
//! never dropped in favour of a longer one. Because the work is now bounded, the route handlers run
//! it under `spawn_blocking` so a large legitimate graph never stalls the async executor.

use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};

use crate::db::queries::pairs as db_pairs;

/// One hop: `(pair_contract, from_asset_id, to_asset_id)`.
pub type PathHop = (String, i32, i32);

type Adjacency = HashMap<i32, Vec<(i32, String)>>;

fn build_adjacency(pair_rows: &[db_pairs::PairRow]) -> Adjacency {
    let mut adj: Adjacency = HashMap::with_capacity(pair_rows.len().saturating_mul(2));
    for p in pair_rows {
        if crate::indexer::asset_code_id_freeze::is_pair_code_id_frozen(&p.contract_address) {
            continue;
        }
        let a0 = p.asset_0_id;
        let a1 = p.asset_1_id;
        adj.entry(a0)
            .or_default()
            .push((a1, p.contract_address.clone()));
        adj.entry(a1)
            .or_default()
            .push((a0, p.contract_address.clone()));
    }
    for edges in adj.values_mut() {
        edges.sort_by(|(left_asset, left_pair), (right_asset, right_pair)| {
            left_pair
                .cmp(right_pair)
                .then_with(|| left_asset.cmp(right_asset))
        });
    }
    adj
}

/// BFS from `goal`: the minimum hop-distance to `goal` for every node within `max_hops` edges.
/// Nodes farther than `max_hops` (or unreachable) are absent from the map. O(V+E).
fn hop_distance_to_goal(goal: i32, adj: &Adjacency, max_hops: usize) -> HashMap<i32, usize> {
    let mut dist: HashMap<i32, usize> = HashMap::new();
    dist.insert(goal, 0);
    let mut q = VecDeque::new();
    q.push_back(goal);
    while let Some(u) = q.pop_front() {
        let du = dist[&u];
        if du >= max_hops {
            continue;
        }
        for (v, _) in adj.get(&u).into_iter().flatten() {
            if !dist.contains_key(v) {
                dist.insert(*v, du + 1);
                q.push_back(*v);
            }
        }
    }
    dist
}

/// Enumerate up to `max_paths` simple paths from `start` to `goal` with at most `max_hops` edges.
/// Returns up to `max_paths` routes, fewest hops first (then lexicographically by pair addresses).
/// Selection is the K shortest via iterative deepening: a route that exists is never silently
/// dropped in favour of a longer one, and an unreachable goal returns empty in O(V+E) (#286).
pub fn find_paths_top_k(
    start: i32,
    goal: i32,
    pair_rows: &[db_pairs::PairRow],
    max_hops: usize,
    max_paths: usize,
) -> Vec<Vec<PathHop>> {
    find_paths_top_k_instrumented(start, goal, pair_rows, max_hops, max_paths).0
}

/// Same as [`find_paths_top_k`] but also returns the number of DFS node expansions, so tests can
/// prove the search is bounded (0 for an unreachable goal, vs full O(branching^max_hops)
/// enumeration without the pruning gate) (#286).
pub(crate) fn find_paths_top_k_instrumented(
    start: i32,
    goal: i32,
    pair_rows: &[db_pairs::PairRow],
    max_hops: usize,
    max_paths: usize,
) -> (Vec<Vec<PathHop>>, u64) {
    if start == goal {
        return (vec![vec![]], 0);
    }
    if max_paths == 0 || max_hops == 0 {
        return (Vec::new(), 0);
    }

    let adj = build_adjacency(pair_rows);

    // Admissible reachability gate: if the goal can't be reached from the start within the hop
    // budget there is no route at all, so return immediately without enumerating. This is the
    // adversarial case from #286 (dense graph, unreachable goal) and also covers an isolated
    // input/output token — both now O(V+E) instead of full simple-path enumeration.
    let dist = hop_distance_to_goal(goal, &adj, max_hops);
    let min_depth = match dist.get(&start) {
        Some(d) if *d <= max_hops => *d,
        _ => return (Vec::new(), 0),
    };

    let mut found: Vec<Vec<PathHop>> = Vec::new();
    let mut expansions: u64 = 0;

    // Enumerate simple paths of *exactly* `depth` edges from `u` to `goal`, pruning any neighbour
    // that cannot itself reach the goal within the edges that remain. Collecting at a fixed depth
    // lets the driver deepen one hop at a time so shortest routes are discovered first.
    #[allow(clippy::too_many_arguments)]
    fn dfs_exact(
        u: i32,
        goal: i32,
        depth: usize,
        max_paths: usize,
        adj: &Adjacency,
        dist: &HashMap<i32, usize>,
        path: &mut Vec<PathHop>,
        visited: &mut Vec<i32>,
        found: &mut Vec<Vec<PathHop>>,
        expansions: &mut u64,
    ) {
        *expansions += 1;
        if found.len() >= max_paths {
            return;
        }
        if path.len() == depth {
            if u == goal {
                found.push(path.clone());
            }
            return;
        }
        // Edges left after stepping to a neighbour, for a route of exactly `depth` edges.
        let remaining = depth - path.len() - 1;
        for (v, pair) in adj.get(&u).into_iter().flatten() {
            if *v == goal {
                // The goal is valid only as the final edge of this depth; reaching it earlier is a
                // shorter route, already enumerated in a previous (shallower) pass.
                if remaining != 0 {
                    continue;
                }
            } else {
                // Admissible prune: skip a neighbour whose own shortest distance to the goal
                // exceeds the remaining budget — it can never lie on a valid depth-length route.
                match dist.get(v) {
                    Some(dv) if *dv <= remaining => {}
                    _ => continue,
                }
            }
            if visited.contains(v) {
                continue;
            }
            visited.push(*v);
            path.push((pair.clone(), u, *v));
            dfs_exact(
                *v, goal, depth, max_paths, adj, dist, path, visited, found, expansions,
            );
            path.pop();
            visited.pop();
            if found.len() >= max_paths {
                return;
            }
        }
    }

    let mut visited = vec![start];
    let mut path = Vec::new();

    // Iterative deepening: take the K SHORTEST routes. Deepening one hop at a time means the
    // `max_paths` cap keeps the fewest-hop routes — the natural execution priority (lower fees, less
    // slippage) — instead of an arbitrary subset, so a route that exists, especially a short/direct
    // one, is never crowded out by longer alternatives (GitLab #286). `dfs_exact` leaves `path`
    // empty and `visited` reset to just `start` between passes, so the buffers are reused safely.
    for depth in min_depth..=max_hops {
        if found.len() >= max_paths {
            break;
        }
        dfs_exact(
            start,
            goal,
            depth,
            max_paths,
            &adj,
            &dist,
            &mut path,
            &mut visited,
            &mut found,
            &mut expansions,
        );
    }

    // Already discovered shortest-first across passes; tie-break deterministically within a hop
    // count by pair-address sequence.
    found.sort_by(|a, b| {
        a.len()
            .cmp(&b.len())
            .then_with(|| cmp_path_pair_addresses(a, b))
    });
    found.truncate(max_paths);
    (found, expansions)
}

fn cmp_path_pair_addresses(left: &[PathHop], right: &[PathHop]) -> Ordering {
    left.iter()
        .map(|(pair, _, _)| pair)
        .cmp(right.iter().map(|(pair, _, _)| pair))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::queries::pairs::PairRow;

    fn mk_pair(id: i32, a0: i32, a1: i32) -> PairRow {
        mk_pair_with_contract(id, format!("pair{id}"), a0, a1)
    }

    fn mk_pair_with_contract(
        id: i32,
        contract_address: impl Into<String>,
        a0: i32,
        a1: i32,
    ) -> PairRow {
        let t = chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0).unwrap();
        PairRow {
            id,
            contract_address: contract_address.into(),
            asset_0_id: a0,
            asset_1_id: a1,
            lp_token: None,
            fee_bps: None,
            hooks: vec![],
            created_at_block: None,
            created_at: t,
            updated_at: t,
        }
    }

    /// Reference enumeration WITHOUT the reachability gate (mirrors the pre-#286 DFS): bounds depth
    /// and result count but has no distance pruning. Used only to demonstrate the blowup the gate
    /// prevents. Returns the DFS node-expansion count.
    fn count_unpruned_expansions(
        start: i32,
        goal: i32,
        pair_rows: &[PairRow],
        max_hops: usize,
        max_paths: usize,
    ) -> u64 {
        let adj = build_adjacency(pair_rows);
        let mut visited = vec![start];

        #[allow(clippy::too_many_arguments)]
        fn go(
            u: i32,
            goal: i32,
            max_hops: usize,
            max_paths: usize,
            depth: usize,
            adj: &Adjacency,
            visited: &mut Vec<i32>,
            found: &mut usize,
            exp: &mut u64,
        ) {
            *exp += 1;
            if *found >= max_paths {
                return;
            }
            if u == goal {
                *found += 1;
                return;
            }
            if depth >= max_hops {
                return;
            }
            for (v, _) in adj.get(&u).into_iter().flatten() {
                if visited.contains(v) {
                    continue;
                }
                visited.push(*v);
                go(
                    *v,
                    goal,
                    max_hops,
                    max_paths,
                    depth + 1,
                    adj,
                    visited,
                    found,
                    exp,
                );
                visited.pop();
            }
        }

        let mut found = 0usize;
        let mut exp = 0u64;
        go(
            start,
            goal,
            max_hops,
            max_paths,
            0,
            &adj,
            &mut visited,
            &mut found,
            &mut exp,
        );
        exp
    }

    fn complete_graph(n: i32) -> Vec<PairRow> {
        let mut pairs = Vec::new();
        let mut id = 1i32;
        for a in 0..n {
            for b in (a + 1)..n {
                pairs.push(mk_pair(id, a, b));
                id += 1;
            }
        }
        pairs
    }

    #[test]
    fn normal_graph_finds_route() {
        // 0-1, 1-2, 2-3 ; find 0 -> 3 within 3 hops.
        let pairs = vec![mk_pair(1, 0, 1), mk_pair(2, 1, 2), mk_pair(3, 2, 3)];
        let paths = find_paths_top_k(0, 3, &pairs, 3, 5);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].len(), 3);
    }

    #[test]
    fn multiple_paths_ordered_shortest_first() {
        // Diamond with a direct edge: 0-3 (1 hop) plus 0-1-3 and 0-2-3 (2 hops).
        let pairs = vec![
            mk_pair(1, 0, 3),
            mk_pair(2, 0, 1),
            mk_pair(3, 1, 3),
            mk_pair(4, 0, 2),
            mk_pair(5, 2, 3),
        ];
        let paths = find_paths_top_k(0, 3, &pairs, 3, 5);
        assert_eq!(paths.len(), 3);
        // Shortest first: the 1-hop direct route leads.
        assert_eq!(paths[0].len(), 1);
        assert!(paths[1..].iter().all(|p| p.len() == 2));
    }

    #[test]
    fn small_unreachable_graph_returns_empty() {
        // Goal absent from every pair: the gate returns immediately, zero DFS expansions.
        let pairs = vec![mk_pair(1, 0, 1), mk_pair(2, 1, 2)];
        let (paths, expansions) = find_paths_top_k_instrumented(0, 7, &pairs, 3, 5);
        assert!(paths.is_empty());
        assert_eq!(expansions, 0, "unreachable goal must not enumerate");
    }

    #[test]
    fn isolated_input_token_returns_empty_immediately() {
        // Plastik's early-exit ask: the input token has no pairs -> unreachable -> no enumeration.
        let pairs = vec![mk_pair(1, 1, 2), mk_pair(2, 2, 3)];
        let (paths, expansions) = find_paths_top_k_instrumented(99, 3, &pairs, 3, 5);
        assert!(paths.is_empty());
        assert_eq!(expansions, 0);
    }

    #[test]
    fn dense_unreachable_graph_does_zero_enumeration() {
        // GitLab #286 abuse case, made concrete: a complete graph on 50 nodes with an UNREACHABLE
        // goal. The reference unpruned walk enumerates every simple path up to max_hops (>100k node
        // expansions) — that is the CPU blowup that blocks the executor. With the reachability gate
        // the goal is absent from the BFS, so the DFS performs ZERO expansions and returns empty.
        let pairs = complete_graph(50);
        let goal = 9999; // in no pair -> unreachable

        let unpruned = count_unpruned_expansions(0, goal, &pairs, 3, 5);
        assert!(
            unpruned > 100_000,
            "demonstration: without the gate this enumerates {unpruned} expansions"
        );

        let (paths, expansions) = find_paths_top_k_instrumented(0, goal, &pairs, 3, 5);
        assert!(paths.is_empty(), "unreachable goal yields no paths");
        assert_eq!(
            expansions, 0,
            "the reachability gate must short-circuit before any enumeration"
        );
    }

    #[test]
    fn reachable_goal_in_dense_graph_is_bounded_and_complete() {
        // Goal IS reachable inside a dense graph. The route is still found (completeness preserved),
        // and the distance pruning keeps expansions far below the unpruned enumeration.
        let mut pairs = complete_graph(40);
        let goal = 40;
        // Attach the goal so it is reachable in 1 hop from node 0 (and 2 hops via others).
        pairs.push(mk_pair(10_000, 0, goal));

        let unpruned = count_unpruned_expansions(0, goal, &pairs, 3, 5);
        let (paths, expansions) = find_paths_top_k_instrumented(0, goal, &pairs, 3, 5);

        assert!(!paths.is_empty(), "a real route must be discovered");
        assert_eq!(paths[0].len(), 1, "the 1-hop direct route is shortest");
        assert!(
            expansions < unpruned,
            "pruning must cut work (pruned {expansions} vs unpruned {unpruned})"
        );
    }

    #[test]
    fn shortest_route_not_crowded_out_by_longer_ones() {
        // start(0) has a DIRECT 1-hop edge to goal(99) plus ten 2-hop routes via intermediates,
        // with the direct edge listed LAST in adjacency order. An arbitrary-order DFS that stops at
        // the first `max_paths` routes fills every slot with 2-hop paths and never discovers the
        // 1-hop direct route — "a path exists and is not discovered" (#286). Iterative deepening
        // takes the fewest-hop routes first, so the direct route is always returned.
        let goal = 99;
        let mut pairs = Vec::new();
        let mut id = 1;
        for k in 1..=10 {
            pairs.push(mk_pair(id, 0, k));
            id += 1;
            pairs.push(mk_pair(id, k, goal));
            id += 1;
        }
        pairs.push(mk_pair(id, 0, goal)); // direct edge, last in adjacency order
        let paths = find_paths_top_k(0, goal, &pairs, 3, 5);
        assert_eq!(paths.len(), 5);
        assert_eq!(
            paths[0].len(),
            1,
            "the 1-hop direct route must be discovered, not crowded out by longer routes"
        );
    }

    #[test]
    fn same_depth_paths_are_capped_in_lexicographic_order() {
        // More shortest paths exist than the cap allows. Adjacency sorting makes the early cap
        // deterministic and keeps the lexicographically first same-hop routes, independent of DB
        // row order.
        let goal = 99;
        let mut pairs = Vec::new();
        let mut id = 1;
        for k in (1..=6).rev() {
            pairs.push(mk_pair_with_contract(id, format!("p{k:02}a"), 0, k));
            id += 1;
            pairs.push(mk_pair_with_contract(id, format!("p{k:02}b"), k, goal));
            id += 1;
        }

        let paths = find_paths_top_k(0, goal, &pairs, 2, 3);
        let keys: Vec<Vec<String>> = paths
            .iter()
            .map(|path| path.iter().map(|(pair, _, _)| pair.clone()).collect())
            .collect();

        assert_eq!(
            keys,
            vec![
                vec!["p01a".to_string(), "p01b".to_string()],
                vec!["p02a".to_string(), "p02b".to_string()],
                vec!["p03a".to_string(), "p03b".to_string()],
            ]
        );
    }

    #[test]
    fn sparse_asset_ids_do_not_allocate_by_max_id() {
        // The visited set is path-depth sized, not max(asset_id) sized. Sparse database IDs should
        // not turn route search into a huge allocation.
        let pairs = vec![
            mk_pair(1, 1_000_000_000, 1_000_000_001),
            mk_pair(2, 1_000_000_001, 1_000_000_002),
        ];

        let paths = find_paths_top_k(1_000_000_000, 1_000_000_002, &pairs, 2, 5);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].len(), 2);
    }

    #[test]
    fn deep_route_within_budget_is_found() {
        // A 3-hop-only route must survive: 0-1-2-3 with no shortcut, max_hops=3.
        let pairs = vec![mk_pair(1, 0, 1), mk_pair(2, 1, 2), mk_pair(3, 2, 3)];
        let paths = find_paths_top_k(0, 3, &pairs, 3, 5);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].len(), 3);
        // And it is correctly excluded when the budget is one hop short.
        let paths2 = find_paths_top_k(0, 3, &pairs, 2, 5);
        assert!(paths2.is_empty());
    }

    #[test]
    fn four_hop_only_route_within_budget_is_found() {
        // A 4-hop-only route must survive: 0-1-2-3-4 with no shortcut, max_hops=4 (#323).
        let pairs = vec![
            mk_pair(1, 0, 1),
            mk_pair(2, 1, 2),
            mk_pair(3, 2, 3),
            mk_pair(4, 3, 4),
        ];
        let paths = find_paths_top_k(0, 4, &pairs, 4, 5);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].len(), 4);
        // Excluded when the budget is one hop short.
        let paths3 = find_paths_top_k(0, 4, &pairs, 3, 5);
        assert!(paths3.is_empty());
    }

    #[test]
    fn unreachable_goal_at_four_hops_does_zero_enumeration() {
        // Reachability gate at max_hops=4: unreachable goal must not enumerate (#286, #323).
        let pairs = complete_graph(50);
        let goal = 9999;
        let (paths, expansions) = find_paths_top_k_instrumented(0, goal, &pairs, 4, 5);
        assert!(paths.is_empty());
        assert_eq!(expansions, 0, "unreachable goal must not enumerate at 4 hops");
    }
}
