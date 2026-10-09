//! Structural paths have a separate bounded cache. Market amounts, reserves,
//! references and fees never supply structural identity or scale executable output.
use super::*;

const MAX_SKELETONS: usize = 128;
type Paths = Vec<Vec<route_paths::PathHop>>;

#[derive(Default)]
pub(super) struct SkeletonCache {
    entries: HashMap<String, Arc<Paths>>,
    hits: u64,
    misses: u64,
}

fn key(s: &Snapshot, start: i32, goal: i32, depth: usize) -> String {
    let mut topology: Vec<_> = s
        .pairs
        .iter()
        .map(|p| {
            (
                &p.contract_address,
                p.asset_0_id,
                p.asset_1_id,
                crate::indexer::asset_code_id_freeze::is_pair_code_id_frozen(&p.contract_address),
            )
        })
        .collect();
    topology.sort();
    let mut digest = Sha256::new();
    digest.update(format!(
        "{}|{topology:?}|{start}|{goal}|{depth}",
        s.namespace
    ));
    format!("{:x}", digest.finalize())
}

pub(super) fn paths(s: &Snapshot, start: i32, goal: i32, depth: usize) -> Paths {
    let depth = depth.min(route_solver::GET_DEFAULT_MAX_HOPS);
    let key = key(s, start, goal, depth);
    {
        let mut cache = store().lock().unwrap();
        if let Some(paths) = cache.skeletons.entries.get(&key).cloned() {
            cache.skeletons.hits += 1;
            return (*paths).clone();
        }
    }
    // Enumeration is CPU-bounded. Concurrent background discovery may race,
    // but publication coalesces identical structure without waiting on I/O.
    let paths = route_paths::find_paths_top_k_with_work_budget(
        start, goal, &s.pairs, depth, 1024, MAX_WORK,
    )
    .0;
    let mut cache = store().lock().unwrap();
    if !cache.skeletons.entries.contains_key(&key) {
        if cache.skeletons.entries.len() >= MAX_SKELETONS {
            cache.skeletons.entries.clear();
        }
        cache.skeletons.misses += 1;
        cache.skeletons.entries.insert(key, Arc::new(paths.clone()));
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_identity_ignores_market_amounts_but_isolates_namespace_and_topology() {
        let mut s = super::super::tests::fixture();
        let initial = key(&s, 1, 2, 2);
        s.generation = "new-market-generation".into();
        s.mirrors.values_mut().for_each(|m| m.reserve_0 *= 2);
        assert_eq!(key(&s, 1, 2, 2), initial);
        let expected = paths(&s, 1, 2, 2);
        assert_eq!(paths(&s, 1, 2, 2), expected);
        assert!(!expected.is_empty());
        s.namespace.push_str("-different-client");
        assert_ne!(key(&s, 1, 2, 2), initial);
        s.namespace = "unit1452".into();
        s.pairs.pop();
        assert_ne!(key(&s, 1, 2, 2), initial);
        assert_ne!(key(&s, 2, 1, 2), initial);
        assert_ne!(key(&s, 1, 2, 4), initial);
    }
}
