//! Simple-path enumeration for multihop route discovery (top-K by hop count).

use std::collections::HashMap;

use crate::db::queries::pairs as db_pairs;

/// One hop: `(pair_contract, from_asset_id, to_asset_id)`.
pub type PathHop = (String, i32, i32);

/// Enumerate up to `max_paths` simple paths from `start` to `goal` with at most `max_hops` edges.
/// Paths are ordered by ascending hop count (shortest first), then lexicographically by pair addresses.
pub fn find_paths_top_k(
    start: i32,
    goal: i32,
    pair_rows: &[db_pairs::PairRow],
    max_hops: usize,
    max_paths: usize,
) -> Vec<Vec<PathHop>> {
    if start == goal {
        return vec![vec![]];
    }
    if max_paths == 0 || max_hops == 0 {
        return Vec::new();
    }

    let mut adj: HashMap<i32, Vec<(i32, String)>> = HashMap::new();
    for p in pair_rows {
        let a0 = p.asset_0_id;
        let a1 = p.asset_1_id;
        adj.entry(a0)
            .or_default()
            .push((a1, p.contract_address.clone()));
        adj.entry(a1)
            .or_default()
            .push((a0, p.contract_address.clone()));
    }

    let mut found: Vec<Vec<PathHop>> = Vec::new();

    fn dfs(
        u: i32,
        goal: i32,
        max_hops: usize,
        max_paths: usize,
        adj: &HashMap<i32, Vec<(i32, String)>>,
        path: &mut Vec<PathHop>,
        on_path: &mut Vec<bool>,
        found: &mut Vec<Vec<PathHop>>,
    ) {
        if found.len() >= max_paths {
            return;
        }
        if u == goal {
            found.push(path.clone());
            return;
        }
        if path.len() >= max_hops {
            return;
        }
        for (v, pair) in adj.get(&u).into_iter().flatten() {
            let vid = *v as usize;
            if vid < on_path.len() && on_path[vid] {
                continue;
            }
            if vid >= on_path.len() {
                on_path.resize(vid + 1, false);
            }
            on_path[vid] = true;
            path.push((pair.clone(), u, *v));
            dfs(*v, goal, max_hops, max_paths, adj, path, on_path, found);
            path.pop();
            on_path[vid] = false;
            if found.len() >= max_paths {
                return;
            }
        }
    }

    let max_node = pair_rows
        .iter()
        .flat_map(|p| [p.asset_0_id, p.asset_1_id])
        .max()
        .unwrap_or(goal)
        .max(start) as usize;
    let mut on_path = vec![false; max_node + 1];
    on_path[start as usize] = true;
    let mut path = Vec::new();
    dfs(
        start,
        goal,
        max_hops,
        max_paths,
        &adj,
        &mut path,
        &mut on_path,
        &mut found,
    );

    found.sort_by(|a, b| {
        a.len()
            .cmp(&b.len())
            .then_with(|| path_sort_key(a).cmp(&path_sort_key(b)))
    });
    found.truncate(max_paths);
    found
}

fn path_sort_key(path: &[PathHop]) -> String {
    path.iter()
        .map(|(pair, _, _)| pair.as_str())
        .collect::<Vec<_>>()
        .join("|")
}
