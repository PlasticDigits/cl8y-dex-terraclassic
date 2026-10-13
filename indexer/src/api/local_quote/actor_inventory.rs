//! Capacity recovery requires the complete canonical factory wallet blacklist at
//! one attested chain height. Partial pages and unknown storage layouts never reset it.
use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
const PAGE_SIZE: usize = 64;
const MAX_PAGES: usize = 128;
const WALLET_PREFIX: &[u8] = b"\0\x0abl_wallets";
#[derive(Default)]
struct Inventory {
    actors: HashMap<String, (u16, bool)>,
    last_key: Option<Vec<u8>>,
    factory_layout: bool,
}
impl Inventory {
    fn add_page(&mut self, value: &Value) -> Option<String> {
        let models = value.get("models")?.as_array()?;
        if models.len() > PAGE_SIZE {
            return None;
        }
        for model in models {
            let key = decode_key(model.get("key")?.as_str()?)?;
            let raw = STANDARD.decode(model.get("value")?.as_str()?).ok()?;
            if self.last_key.as_ref().is_some_and(|last| last >= &key) {
                return None;
            }
            self.last_key = Some(key.clone());
            if key == b"contract_info" {
                let cw2: Value = serde_json::from_slice(&raw).ok()?;
                self.factory_layout = cw2["contract"].as_str() == Some("cl8y-dex-factory")
                    && cw2["version"].as_str() == Some("1.12.0");
                if !self.factory_layout {
                    return None;
                }
            }
            if let Some(address) = key.strip_prefix(WALLET_PREFIX) {
                let address = std::str::from_utf8(address).ok()?;
                if !address.starts_with("terra1")
                    || address.len() > 128
                    || serde_json::from_slice::<bool>(&raw).ok() != Some(true)
                {
                    return None;
                }
                if self.actors.len() >= super::ingestion::MAX_ACTOR_CONTEXTS {
                    return None;
                }
                if self.actors.insert(address.into(), (0, true)).is_some() {
                    return None;
                }
            }
        }
        let next = value.get("pagination")?.get("next_key")?;
        if next.is_null() {
            Some(String::new())
        } else {
            Some(next.as_str()?.to_string())
        }
    }
}
// Wasmd Model.key is HexBytes, while values and pagination cursors are base64.
fn decode_key(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|chunk| {
            let hi = (chunk[0] as char).to_digit(16)?;
            let lo = (chunk[1] as char).to_digit(16)?;
            Some((hi * 16 + lo) as u8)
        })
        .collect()
}
fn escaped_key(key: &str) -> String {
    key.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
async fn observe(
    lcd: &crate::lcd::LcdClient,
    factory: &str,
) -> Option<HashMap<String, (u16, bool)>> {
    lcd.require_verified_chain().await.ok()?;
    let height = lcd.get_latest_block_height_bounded(16_384).await.ok()?;
    let mut inventory = Inventory::default();
    let mut next = String::new();
    let mut cursors = HashSet::new();
    for _ in 0..MAX_PAGES {
        let path=format!("/cosmwasm/wasm/v1/contract/{factory}/state?pagination.limit={PAGE_SIZE}&pagination.key={}",escaped_key(&next));
        // get_at_height caps the response and rejects a missing/mismatched height header.
        let page: Value = lcd.get_at_height(&path, height).await.ok()?;
        next = inventory.add_page(&page)?;
        if next.is_empty() {
            return inventory.factory_layout.then_some(inventory.actors);
        }
        STANDARD.decode(&next).ok()?;
        if !cursors.insert(next.clone()) {
            return None;
        }
    }
    None
}
/// Only an independently scheduled admission pass may invoke recovery. The
/// terminal page and layout proof are necessary before the atomic reset.
pub(super) async fn recover(state: &AppState) -> bool {
    let namespace = state.lcd.chain_cache_namespace();
    let (expected_epoch, expected_guard_epoch) = {
        let g = store().lock().unwrap();
        if !g.actor_capacity_exhausted.contains(&namespace) {
            return true;
        }
        (
            g.snapshots.get(&namespace).map(|s| s.config_epoch),
            g.next_config_epoch,
        )
    };
    let Some(factory) = state.factory_address.as_deref() else {
        return false;
    };
    let actors = tokio::time::timeout(Duration::from_secs(45), observe(&state.lcd, factory))
        .await
        .ok()
        .flatten();
    let Some(actors) = actors else {
        return false;
    };
    let mut g = store().lock().unwrap();
    if g.next_config_epoch != expected_guard_epoch {
        return false;
    }
    apply_recovery(&mut g, &namespace, actors, expected_epoch)
}
pub(super) fn apply_recovery(
    g: &mut Store,
    namespace: &str,
    actors: HashMap<String, (u16, bool)>,
    expected_epoch: Option<u64>,
) -> bool {
    if g.snapshots.get(namespace).map(|s| s.config_epoch) != expected_epoch {
        return false;
    }
    if actors.len() > super::ingestion::MAX_ACTOR_CONTEXTS {
        return false;
    }
    if g.early_actor_blocks.len() >= 32 && !g.early_actor_blocks.contains_key(namespace) {
        g.actor_capacity_registry_exhausted = true;
        return false;
    }
    if actor_versions::authoritative_inventory(g, namespace, &actors).is_err() {
        return false;
    }
    let blocks = actors.keys().cloned().collect();
    if let Some(previous) = g.snapshots.get(namespace).cloned() {
        let mut snapshot = (*previous).clone();
        snapshot.actors = actors;
        let mut ordered = snapshot.actors.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|(a, _)| *a);
        snapshot.generation = format!(
            "{:x}",
            Sha256::digest(format!(
                "{}|authoritative-wallets|{ordered:?}",
                snapshot.generation
            ))
        );
        g.next_config_epoch = g.next_config_epoch.saturating_add(1);
        snapshot.config_epoch = g.next_config_epoch;
        snapshot.actor_revision = actor_versions::revision(g, namespace);
        g.snapshots.insert(namespace.to_owned(), Arc::new(snapshot));
    } else {
        // The complete inventory also supplies cold guards before any economic
        // initialization. A partial scan never reaches this replacement.
        g.next_config_epoch = g.next_config_epoch.saturating_add(1);
    }
    g.early_actor_blocks.insert(namespace.to_owned(), blocks);
    g.results.clear();
    g.actor_capacity_exhausted.remove(namespace);
    true
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[serial_test::serial]
    async fn issue1489_complete_inventory_revokes_pending_admission_discount() {
        let namespace = "inventory-pending-admission1489";
        let actor = publication_tests::address(31000);
        let mut snapshot = super::super::tests::fixture();
        snapshot.namespace = namespace.into();
        snapshot.actors.insert(actor.clone(), (800, false));
        publication_tests::pending(snapshot.clone())
            .scheduled()
            .await
            .unwrap();
        ingestion::publish_known_unblock(namespace, &actor).unwrap();
        let pending = publication_tests::pending(snapshot);
        assert!(pending.expected_config_epoch.is_none());
        {
            let mut g = store().lock().unwrap();
            let epoch = g.snapshots[namespace].config_epoch;
            assert!(apply_recovery(
                &mut g,
                namespace,
                HashMap::new(),
                Some(epoch)
            ));
            assert!(g.snapshots[namespace].actors.is_empty());
        }
        pending.scheduled().await.unwrap();
        let actual = store().lock().unwrap().snapshots[namespace]
            .actors
            .get(&actor)
            .copied()
            .unwrap_or((0, false));
        {
            let mut g = store().lock().unwrap();
            g.snapshots.remove(namespace);
            g.actor_versions.remove(namespace);
            g.early_actor_blocks.remove(namespace);
        }
        assert_eq!(
            actual,
            (0, false),
            "late admission must not restore a reset discount"
        );
    }
    fn page(entries: Vec<(Vec<u8>, Vec<u8>)>, next: Option<&str>) -> Value {
        json!({"models":entries.into_iter().map(|(k,v)|json!({"key":k.iter().map(|b|format!("{b:02X}")).collect::<String>(),"value":STANDARD.encode(v)})).collect::<Vec<_>>(),"pagination":{"next_key":next}})
    }
    fn layout() -> (Vec<u8>, Vec<u8>) {
        (
            b"contract_info".to_vec(),
            br#"{"contract":"cl8y-dex-factory","version":"1.12.0"}"#.to_vec(),
        )
    }
    #[test]
    fn complete_inventory_requires_canonical_layout_order_and_positive_blocks() {
        let mut inventory = Inventory::default();
        let mut key = WALLET_PREFIX.to_vec();
        key.extend_from_slice(b"terra1blocked");
        assert_eq!(
            inventory.add_page(&page(vec![(key, b"true".to_vec())], Some("Y3Vyc29y"))),
            Some("Y3Vyc29y".into())
        );
        assert!(!inventory.factory_layout);
        assert_eq!(
            inventory.add_page(&page(vec![layout()], None)),
            Some(String::new())
        );
        assert!(inventory.factory_layout);
        assert_eq!(inventory.actors["terra1blocked"], (0, true));
        assert!(inventory.add_page(&page(vec![layout()], None)).is_none());
        assert!(Inventory::default()
            .add_page(&json!({"models":[]}))
            .is_none());
        let wrong = (
            b"contract_info".to_vec(),
            br#"{"contract":"foreign","version":"1"}"#.to_vec(),
        );
        assert!(Inventory::default()
            .add_page(&page(vec![wrong], None))
            .is_none());
    }
    #[test]
    fn capacity_and_malformed_storage_fail_closed() {
        let mut inventory = Inventory::default();
        inventory.actors = (0..super::ingestion::MAX_ACTOR_CONTEXTS)
            .map(|i| (format!("terra1{i}"), (0, true)))
            .collect();
        let mut key = WALLET_PREFIX.to_vec();
        key.extend_from_slice(b"terra1additional");
        assert!(inventory
            .add_page(&page(vec![(key, b"true".to_vec())], None))
            .is_none());
        assert_eq!(escaped_key("a+/="), "a%2B%2F%3D");
    }
    #[test]
    fn authoritative_recovery_replaces_negatives_invalidates_cache_and_preserves_global_overflow() {
        let namespace = "recovery-fixture";
        let mut g = Store::default();
        let mut snapshot = super::super::tests::fixture();
        snapshot.actors.insert("terra1old".into(), (0, true));
        let old_generation = snapshot.generation.clone();
        g.snapshots.insert(namespace.into(), Arc::new(snapshot));
        mark_actor_capacity_exhausted(&mut g, namespace);
        g.actor_capacity_registry_exhausted = true;
        g.results.insert("old-cache".into(), json!({"quote":"old"}));
        assert!(apply_recovery(
            &mut g,
            namespace,
            HashMap::from([("terra1current".into(), (0, true))]),
            Some(0)
        ));
        assert!(!g.actor_capacity_exhausted.contains(namespace));
        assert!(g.actor_capacity_registry_exhausted);
        assert!(g.results.is_empty());
        let current = &g.snapshots[namespace];
        assert!(!current.actors.contains_key("terra1old"));
        assert_eq!(current.actors["terra1current"], (0, true));
        assert_ne!(current.generation, old_generation);
    }
    #[test]
    fn complete_inventory_supplies_cold_guards_without_economic_initialization() {
        let mut g = Store::default();
        g.early_actor_blocks
            .insert("cold-inventory".into(), HashSet::from(["old-block".into()]));
        mark_actor_capacity_exhausted(&mut g, "cold-inventory");
        assert!(apply_recovery(
            &mut g,
            "cold-inventory",
            HashMap::from([("current-block".into(), (0, true))]),
            None
        ));
        assert!(g.snapshots.is_empty());
        assert_eq!(
            g.early_actor_blocks["cold-inventory"],
            HashSet::from(["current-block".into()])
        );
        assert!(!actor_capacity_unavailable(&g, "cold-inventory"));
        assert!(g.next_config_epoch > 0);
    }
    #[test]
    fn factory_inventory_cannot_clear_unrelated_tax_wallet_ledger_quarantine() {
        let mut g = Store::default();
        g.snapshots.insert(
            "tax-ledger-overflow".into(),
            Arc::new(super::super::tests::fixture()),
        );
        mark_actor_capacity_exhausted(&mut g, "tax-ledger-overflow");
        mark_tax_wallet_capacity_exhausted(&mut g, "tax-ledger-overflow");
        assert!(apply_recovery(
            &mut g,
            "tax-ledger-overflow",
            HashMap::new(),
            Some(0)
        ));
        assert!(!g.actor_capacity_exhausted.contains("tax-ledger-overflow"));
        assert!(g
            .tax_wallet_capacity_exhausted
            .contains("tax-ledger-overflow"));
        assert!(actor_capacity_unavailable(&g, "tax-ledger-overflow"));
    }
    #[tokio::test]
    async fn complete_pinned_height_provider_scan_and_missing_height_fail_closed() {
        use wiremock::matchers::{header, method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        for response_height in [Some("1452"), None] {
            let server = MockServer::start().await;
            crate::lcd::mock_chain_identity(&server, "columbus-5").await;
            Mock::given(method("GET")).and(path("/cosmos/base/tendermint/v1beta1/blocks/latest"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({"block":{"header":{"height":"1452","chain_id":"columbus-5","time":"2026-10-09T00:00:00Z"},"data":{"txs":[]}}})))
                .mount(&server).await;
            let mut key = WALLET_PREFIX.to_vec();
            key.extend_from_slice(b"terra1current");
            let mut response = ResponseTemplate::new(200)
                .set_body_json(page(vec![(key, b"true".to_vec()), layout()], None));
            if let Some(height) = response_height {
                response = response.insert_header("x-cosmos-block-height", height);
            }
            Mock::given(method("GET"))
                .and(path("/cosmwasm/wasm/v1/contract/terra1factory/state"))
                .and(query_param("height", "1452"))
                .and(header("x-cosmos-block-height", "1452"))
                .respond_with(response)
                .mount(&server)
                .await;
            let lcd = crate::lcd::LcdClient::new(vec![server.uri()], 1000, 1, "columbus-5");
            let observed = observe(&lcd, "terra1factory").await;
            if response_height.is_some() {
                assert_eq!(observed.unwrap()["terra1current"], (0, true));
            } else {
                assert!(observed.is_none());
            }
        }
    }
    #[test]
    fn older_complete_inventory_cannot_erase_newer_actor_observation() {
        let namespace = "recovery-race";
        let mut g = Store::default();
        let mut snapshot = super::super::tests::fixture();
        snapshot.config_epoch = 2;
        snapshot
            .actors
            .insert("terra1newer-block".into(), (0, true));
        g.snapshots.insert(namespace.into(), Arc::new(snapshot));
        mark_actor_capacity_exhausted(&mut g, namespace);
        g.results
            .insert("newer-cache".into(), json!({"generation":"newer"}));
        assert!(!apply_recovery(&mut g, namespace, HashMap::new(), Some(1)));
        assert!(g.actor_capacity_exhausted.contains(namespace));
        assert!(g.snapshots[namespace].actors["terra1newer-block"].1);
        assert!(g.results.contains_key("newer-cache"));
    }
}
