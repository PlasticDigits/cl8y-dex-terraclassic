//! Bounded, process-owned fee generation provenance. Only scheduled producers
//! write phases; quotes read immutable completed mirrors and never enqueue work.
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::api::db_orderbook_sim::{HopMirror, MirrorFreshness};
use chrono::{DateTime, Utc};

const MAX_NAMESPACES: usize = 32;
const MAX_MARKERS: usize = 512;
const MAX_COMPLETE: usize = 128;
const MAX_COMPLETE_BYTES: usize = 4 * 1024 * 1024;
const MAX_COMPLETE_ORDERS: usize = 4096;
const PHASE_BOUND: Duration = Duration::from_secs(4);
const RETENTION_AGE_MS: u64 = 20_000;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Stamp {
    height: Option<i64>,
    at_us: i64,
}
impl Stamp {
    fn covers(self, older: Self) -> bool {
        self.at_us >= older.at_us
            && match (self.height, older.height) {
                (Some(new), Some(old)) => new >= old,
                (_, None) => true,
                (None, Some(_)) => false,
            }
    }
}
#[derive(Clone)]
struct Complete {
    mirror: Arc<HopMirror>,
    expiry_ms: i64,
    max_age_ms: u64,
    bytes: usize,
    orders: usize,
}
#[derive(Clone, Copy)]
struct Phase {
    stamp: Stamp,
    reserves: (u128, u128),
    pending: bool,
    until: Instant,
}
struct Entry {
    context: String,
    qualified: bool,
    assets: (i32, i32),
    expiry_ms: i64,
    safety_age_ms: u64,
    context_version: u64,
    owner: u64,
    phase: Option<Phase>,
    negative: bool,
    watermark: Option<Stamp>,
    validated: Option<Stamp>,
    published: Option<Stamp>,
    pool_published: Option<Stamp>,
    qualified_fee: Option<u16>,
    complete: Option<Complete>,
    pool_complete: Option<Complete>,
}
#[derive(Default)]
struct Namespace {
    revision: u64,
    entries: HashMap<i32, Entry>,
}
#[derive(Default)]
struct Registry {
    namespaces: HashMap<String, Namespace>,
    next: u64,
}
fn registry() -> &'static Mutex<Registry> {
    static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(Registry::default()))
}
fn next(reg: &mut Registry) -> u64 {
    reg.next = reg.next.checked_add(1).expect("fee phase epoch exhausted");
    reg.next
}
fn stamp(mirror: &HopMirror) -> Stamp {
    Stamp {
        height: mirror.block_height,
        at_us: mirror.snapshot_at.timestamp_micros(),
    }
}
fn live_tombstone(entry: &Entry, now: i64) -> bool {
    entry.negative
        && entry.watermark.is_some_and(|stamp| {
            stamp
                .at_us
                .div_euclid(1000)
                .saturating_add(i64::try_from(entry.safety_age_ms).unwrap_or(i64::MAX))
                .saturating_add(2)
                > now
        })
}
fn active_provenance(entry: &Entry, now: i64) -> bool {
    live_tombstone(entry, now)
        || entry
            .phase
            .is_some_and(|phase| phase.until > Instant::now())
}
fn prune(reg: &mut Registry) {
    let now = Utc::now().timestamp_millis();
    let instant = Instant::now();
    for namespace in reg.namespaces.values_mut() {
        for entry in namespace.entries.values_mut() {
            if entry.phase.is_some_and(|phase| phase.until <= instant) {
                let phase = entry.phase.take().unwrap();
                entry.negative = true;
                entry.watermark = Some(phase.stamp);
                entry.complete = None;
                entry.pool_complete = None;
                namespace.revision = namespace.revision.saturating_add(1);
            }
            for complete in [&mut entry.complete, &mut entry.pool_complete] {
                if complete.as_ref().is_some_and(|complete| {
                    complete.expiry_ms <= now
                        || crate::api::db_orderbook_sim::is_snapshot_stale(
                            complete.mirror.snapshot_at,
                            complete.max_age_ms,
                        )
                }) {
                    *complete = None;
                }
            }
        }
        namespace
            .entries
            .retain(|_, entry| entry.expiry_ms > now || active_provenance(entry, now));
    }
    reg.namespaces
        .retain(|_, namespace| !namespace.entries.is_empty());
}

/// One currently admitted pair; unsupported inventory never enters this registry.
pub(crate) struct Qualification {
    pub pair_id: i32,
    pub assets: (i32, i32),
    pub context: String,
    pub expiry_ms: i64,
    pub max_age_ms: u64,
    pub mirror: Option<Arc<HopMirror>>,
    pub pool_mirror: Option<Arc<HopMirror>>,
}
fn mirror_cost(mirror: &HopMirror) -> Option<(usize, usize)> {
    if mirror.reserve_0 == 0
        || mirror.reserve_1 == 0
        || mirror.fee_bps > 10_000
        || mirror.block_height.is_none_or(|height| height <= 0)
        || !matches!(mirror.freshness, MirrorFreshness::Fresh)
        || mirror.asset_0_addr.len() > 128
        || mirror.asset_1_addr.len() > 128
        || mirror.bids.len() > 500
        || mirror.asks.len() > 500
    {
        return None;
    }
    let mut bytes = std::mem::size_of::<HopMirror>()
        + mirror.asset_0_addr.capacity()
        + mirror.asset_1_addr.capacity()
        + (mirror.bids.capacity() + mirror.asks.capacity())
            * std::mem::size_of::<crate::db::queries::resting_orders::RestingOrderRow>();
    for order in mirror.bids.iter().chain(&mirror.asks) {
        // Bound numeric work before formatting PostgreSQL values. A retained
        // book uses only the same reachable Uint128/18-place values as quotes.
        let (price, scale) = order.price.as_bigint_and_exponent();
        let (remaining, remaining_scale) = order.remaining.as_bigint_and_exponent();
        if price.bits() > 256
            || !(-38..=18).contains(&scale)
            || remaining.bits() > 128
            || !(-38..=0).contains(&remaining_scale)
            || order.owner.as_ref().is_some_and(|owner| owner.len() > 128)
        {
            return None;
        }
        let price = &order.price * bigdecimal::BigDecimal::from(10u128.pow(18));
        let price_raw = price.with_scale(0).to_string().parse::<u128>().ok()?;
        let remaining_raw = order
            .remaining
            .with_scale(0)
            .to_string()
            .parse::<u128>()
            .ok()?;
        if price_raw == 0
            || price != bigdecimal::BigDecimal::from(price_raw)
            || order.remaining != bigdecimal::BigDecimal::from(remaining_raw)
        {
            return None;
        }
        bytes = bytes.checked_add(256 + order.owner.as_ref().map_or(0, String::capacity))?;
    }
    Some((bytes, mirror.bids.len() + mirror.asks.len()))
}
/// Called under snapshot publication ownership. Does not clear a negative;
/// only the producer's matching completed fee can establish recovery.
pub(crate) fn qualify(namespace: &str, qualified: Vec<Qualification>) -> bool {
    if namespace.len() > 256 || qualified.len() > 512 {
        return false;
    }
    let mut reg = registry().lock().unwrap();
    prune(&mut reg);
    if qualified.is_empty() {
        if let Some(ns) = reg.namespaces.get_mut(namespace) {
            ns.entries
                .retain(|_, entry| active_provenance(entry, Utc::now().timestamp_millis()));
            for entry in ns.entries.values_mut() {
                entry.qualified = false;
            }
        }
        return true;
    }
    if !reg.namespaces.contains_key(namespace) && reg.namespaces.len() >= MAX_NAMESPACES {
        return false;
    }
    let ids = qualified
        .iter()
        .map(|q| q.pair_id)
        .collect::<std::collections::HashSet<_>>();
    if let Some(ns) = reg.namespaces.get_mut(namespace) {
        ns.entries.retain(|id, entry| {
            ids.contains(id) || active_provenance(entry, Utc::now().timestamp_millis())
        });
        for (id, entry) in ns.entries.iter_mut() {
            entry.qualified = ids.contains(id);
        }
    }
    let existing = reg
        .namespaces
        .values()
        .map(|ns| ns.entries.len())
        .sum::<usize>();
    let new_count = qualified
        .iter()
        .filter(|q| {
            reg.namespaces
                .get(namespace)
                .is_none_or(|ns| !ns.entries.contains_key(&q.pair_id))
        })
        .count();
    if existing + new_count > MAX_MARKERS
        || qualified
            .iter()
            .any(|q| q.context.len() > 128 || q.context.is_empty())
    {
        return false;
    }
    for q in qualified {
        let changed = reg
            .namespaces
            .get(namespace)
            .and_then(|ns| ns.entries.get(&q.pair_id))
            .is_none_or(|entry| entry.context != q.context || entry.assets != q.assets);
        let version = if changed { next(&mut reg) } else { 0 };
        let ns = reg.namespaces.entry(namespace.to_owned()).or_default();
        let entry = ns.entries.entry(q.pair_id).or_insert_with(|| Entry {
            context: q.context.clone(),
            qualified: true,
            assets: q.assets,
            expiry_ms: q.expiry_ms,
            safety_age_ms: q.max_age_ms,
            context_version: version,
            owner: 0,
            phase: None,
            negative: false,
            watermark: None,
            validated: None,
            published: None,
            pool_published: None,
            qualified_fee: None,
            complete: None,
            pool_complete: None,
        });
        if changed {
            entry.context = q.context;
            entry.assets = q.assets;
            entry.context_version = version;
            entry.phase = None;
            entry.owner = 0;
            entry.complete = None;
            entry.pool_complete = None;
            entry.qualified_fee = None;
            ns.revision = ns.revision.saturating_add(1);
        }
        entry.qualified = true;
        entry.expiry_ms = q.expiry_ms;
        entry.safety_age_ms = entry.safety_age_ms.max(q.max_age_ms);
        // Each lane advances independently. A newer complete pool must not make
        // the still-compatible original hybrid unusable, or relabel its book.
        let paired = q.pool_mirror.is_some();
        for (pool_view, mirror) in [(true, q.pool_mirror), (false, q.mirror)] {
            let Some(mirror) = mirror else { continue };
            let candidate = stamp(&mirror);
            let entry = reg
                .namespaces
                .get_mut(namespace)
                .unwrap()
                .entries
                .get_mut(&q.pair_id)
                .unwrap();
            let published = if pool_view {
                entry.pool_published
            } else {
                entry.published
            };
            if entry.negative
                || published.is_some_and(|published| !candidate.covers(published))
                || ((pool_view || !paired)
                    && entry
                        .validated
                        .is_some_and(|validated| !candidate.covers(validated)))
                || entry
                    .watermark
                    .is_some_and(|watermark| !candidate.covers(watermark))
                || (!pool_view && paired && entry.qualified_fee != Some(mirror.fee_bps))
                || crate::api::db_orderbook_sim::is_snapshot_stale(
                    mirror.snapshot_at,
                    q.max_age_ms.min(RETENTION_AGE_MS),
                )
            {
                continue;
            }
            if mirror.reserve_0 > 0
                && mirror.reserve_1 > 0
                && mirror.fee_bps <= 10_000
                && mirror.block_height.is_some_and(|height| height > 0)
            {
                entry.qualified_fee = Some(mirror.fee_bps);
                if entry
                    .validated
                    .is_none_or(|validated| candidate.covers(validated))
                {
                    entry.validated = Some(candidate);
                }
                if pool_view {
                    entry.pool_published = Some(candidate);
                } else {
                    entry.published = Some(candidate);
                }
            }
            let saved = if pool_view {
                &mut entry.pool_complete
            } else {
                &mut entry.complete
            };
            if saved
                .as_ref()
                .is_some_and(|old| stamp(&old.mirror) == candidate)
            {
                continue;
            }
            let Some((bytes, orders)) = mirror_cost(&mirror) else {
                continue;
            };
            *saved = None;
            let totals = reg
                .namespaces
                .values()
                .flat_map(|ns| ns.entries.values())
                .flat_map(|entry| [&entry.complete, &entry.pool_complete])
                .filter_map(Option::as_ref)
                .fold((0usize, 0usize, 0usize), |(count, b, o), c| {
                    (count + 1, b + c.bytes, o + c.orders)
                });
            if totals.0 >= MAX_COMPLETE
                || totals.1 + bytes > MAX_COMPLETE_BYTES
                || totals.2 + orders > MAX_COMPLETE_ORDERS
            {
                continue;
            }
            let entry = reg
                .namespaces
                .get_mut(namespace)
                .unwrap()
                .entries
                .get_mut(&q.pair_id)
                .unwrap();
            let mirror = Arc::new((*mirror).clone());
            let (bytes, orders) =
                mirror_cost(&mirror).expect("compact clone preserves validated bounds");
            let complete = Some(Complete {
                mirror,
                expiry_ms: q.expiry_ms,
                max_age_ms: q.max_age_ms.min(RETENTION_AGE_MS),
                bytes,
                orders,
            });
            if pool_view {
                entry.pool_complete = complete;
            } else {
                entry.complete = complete;
            }
        }
    }
    true
}

/// Producer ownership starts before raw publication, after exact asset and
/// positive reserve validation. Unknown/capacity provenance cannot enable reuse.
pub fn begin(
    namespace: &str,
    pair_id: i32,
    assets: (i32, i32),
    height: i64,
    observed_at: DateTime<Utc>,
    reserves: (u128, u128),
) -> Option<FeePhaseGuard> {
    if height <= 0 || reserves.0 == 0 || reserves.1 == 0 || observed_at > Utc::now() {
        reject(namespace, pair_id, Some(height), observed_at);
        return None;
    }
    let mut reg = registry().lock().unwrap();
    prune(&mut reg);
    let source = reg.namespaces.get(namespace)?.entries.get(&pair_id)?;
    let incoming = Stamp {
        height: Some(height),
        at_us: observed_at.timestamp_micros(),
    };
    if !source.qualified || source.expiry_ms <= Utc::now().timestamp_millis() {
        return None;
    }
    if source.assets != assets {
        drop(reg);
        reject(namespace, pair_id, Some(height), observed_at);
        return None;
    }
    if source
        .phase
        .is_some_and(|phase| !incoming.covers(phase.stamp))
        || source.validated.is_some_and(|old| !incoming.covers(old))
    {
        return None;
    }
    let context_version = source.context_version;
    let owner = next(&mut reg);
    let ns = reg.namespaces.get_mut(namespace).unwrap();
    ns.entries.get_mut(&pair_id).unwrap().owner = owner;
    ns.entries.get_mut(&pair_id).unwrap().phase = Some(Phase {
        stamp: incoming,
        reserves,
        pending: true,
        until: Instant::now() + PHASE_BOUND,
    });
    Some(FeePhaseGuard {
        namespace: namespace.to_owned(),
        pair_id,
        context_version,
        owner,
        stamp: incoming,
        resolved: false,
    })
}

/// Safety ownership exists even when cold quote qualification has not created a
/// registry entry. This guard never allocates marker or retained-mirror state.
/// Quote publication independently rejects incomplete raw generations without
/// an already qualified pending proof.
pub struct UntrackedFeeGuard {
    namespace: String,
    pair_id: i32,
    assets: (i32, i32),
    stamp: Stamp,
    resolved: bool,
}
impl UntrackedFeeGuard {
    pub fn new(
        namespace: &str,
        pair_id: i32,
        assets: (i32, i32),
        height: i64,
        at: DateTime<Utc>,
    ) -> Self {
        Self {
            namespace: namespace.to_owned(),
            pair_id,
            assets,
            stamp: Stamp {
                height: Some(height),
                at_us: at.timestamp_micros(),
            },
            resolved: false,
        }
    }
    fn reject(&self) {
        let at = DateTime::<Utc>::from_timestamp_micros(self.stamp.at_us)
            .expect("stamp originates from a valid DateTime");
        reject(&self.namespace, self.pair_id, self.stamp.height, at);
    }
    pub fn safety_callback(&self) -> Arc<dyn Fn() + Send + Sync> {
        let namespace = self.namespace.clone();
        let pair_id = self.pair_id;
        let stamp = self.stamp;
        Arc::new(move || {
            let at = DateTime::<Utc>::from_timestamp_micros(stamp.at_us)
                .expect("stamp originates from a valid DateTime");
            reject(&namespace, pair_id, stamp.height, at);
        })
    }
    pub fn finish_validated_fee(mut self, fee: u16) {
        self.resolved = true;
        let same_known_fee = {
            let reg = registry().lock().unwrap();
            reg.namespaces
                .get(&self.namespace)
                .and_then(|ns| ns.entries.get(&self.pair_id))
                .is_some_and(|entry| {
                    entry.assets == self.assets && entry.qualified_fee == Some(fee)
                })
        };
        // Unknown or changed economics cannot be labelled with an old hybrid.
        // Do not adopt a later context or clear a newer negative without an owner.
        if fee > 10_000 || !same_known_fee {
            self.reject();
        }
    }
    pub fn finish_transient(mut self) {
        self.resolved = true;
    }
    pub fn finish_negative(mut self) {
        self.resolved = true;
        self.reject();
    }
}
impl Drop for UntrackedFeeGuard {
    fn drop(&mut self) {
        if !self.resolved {
            self.reject();
        }
    }
}

pub enum ProducerFeeGuard {
    Qualified(FeePhaseGuard),
    Untracked(UntrackedFeeGuard),
}
impl ProducerFeeGuard {
    pub fn safety_callback(&self) -> Arc<dyn Fn() + Send + Sync> {
        match self {
            Self::Qualified(g) => g.safety_callback(),
            Self::Untracked(g) => g.safety_callback(),
        }
    }
    pub fn finish_validated_fee(self, fee: u16) {
        match self {
            Self::Qualified(g) => g.finish_validated_fee(fee),
            Self::Untracked(g) => g.finish_validated_fee(fee),
        }
    }
    pub fn finish_transient(self) {
        match self {
            Self::Qualified(g) => g.finish_transient(),
            Self::Untracked(g) => g.finish_transient(),
        }
    }
    pub fn finish_negative(self) {
        match self {
            Self::Qualified(g) => g.finish_negative(),
            Self::Untracked(g) => g.finish_negative(),
        }
    }
}

pub struct FeePhaseGuard {
    namespace: String,
    pair_id: i32,
    context_version: u64,
    owner: u64,
    stamp: Stamp,
    resolved: bool,
}
impl FeePhaseGuard {
    pub fn finish_validated(mut self) {
        self.finish(Outcome::Validated);
    }
    /// A changed authoritative fee revokes old positive economics immediately;
    /// unchanged fees preserve the ordinary original-age mirror until publication.
    pub fn finish_validated_fee(mut self, fee: u16) {
        if fee > 10_000 {
            self.finish(Outcome::Negative);
            return;
        }
        let changed = {
            let reg = registry().lock().unwrap();
            reg.namespaces
                .get(&self.namespace)
                .and_then(|ns| ns.entries.get(&self.pair_id))
                .filter(|entry| {
                    entry.owner == self.owner && entry.context_version == self.context_version
                })
                .is_none_or(|entry| entry.qualified_fee != Some(fee))
        };
        if changed {
            self.finish(Outcome::Negative);
        }
        self.finish(Outcome::Validated);
    }
    pub fn finish_transient(mut self) {
        self.finish(Outcome::Transient);
    }
    pub fn finish_negative(mut self) {
        self.finish(Outcome::Negative);
    }
    /// Attached only to this owner's fee LCD view. A known negative is visible
    /// to quotes even while LCD metadata cleanup cannot acquire its write lock.
    pub fn safety_callback(&self) -> Arc<dyn Fn() + Send + Sync> {
        let namespace = self.namespace.clone();
        let pair_id = self.pair_id;
        let context_version = self.context_version;
        let owner = self.owner;
        let stamp = self.stamp;
        Arc::new(move || {
            let mut owner = FeePhaseGuard {
                namespace: namespace.clone(),
                pair_id,
                context_version,
                owner,
                stamp,
                resolved: true,
            };
            owner.finish(Outcome::Negative);
        })
    }
    fn finish(&mut self, outcome: Outcome) {
        self.resolved = true;
        let mut reg = registry().lock().unwrap();
        prune(&mut reg);
        let Some(ns) = reg.namespaces.get_mut(&self.namespace) else {
            return;
        };
        let Some(entry) = ns.entries.get_mut(&self.pair_id) else {
            return;
        };
        if entry.context_version != self.context_version {
            return;
        }
        let owns = entry.owner == self.owner;
        if matches!(outcome, Outcome::Negative) {
            if entry
                .validated
                .is_some_and(|validated| !self.stamp.covers(validated))
            {
                return;
            }
            if entry.negative && entry.watermark == Some(self.stamp) {
                if owns {
                    entry.phase = None;
                }
                return;
            }
            entry.negative = true;
            entry.complete = None;
            entry.pool_complete = None;
            if entry.watermark.is_none_or(|old| self.stamp.covers(old)) {
                entry.watermark = Some(self.stamp);
            }
            if owns {
                entry.phase = None;
            }
            ns.revision = ns.revision.saturating_add(1);
            drop(reg);
            crate::api::local_quote::invalidate_fee_generation(&self.namespace, self.pair_id);
        } else if owns {
            if matches!(outcome, Outcome::Validated) {
                if entry
                    .watermark
                    .is_some_and(|watermark| !self.stamp.covers(watermark))
                {
                    return;
                }
                entry.negative = false;
                entry.validated = Some(self.stamp);
                entry.phase = None;
            } else if let Some(phase) = entry.phase.as_mut() {
                phase.pending = false;
                // Preserve the original marker deadline; completion is not renewal.
            }
            // Positive producer completion does not change a published tuple.
            // Changed fees already take the Negative branch above; actual tuple
            // publication and context qualification own economic invalidation.
        }
    }
}
enum Outcome {
    Validated,
    Transient,
    Negative,
}
impl Drop for FeePhaseGuard {
    fn drop(&mut self) {
        if !self.resolved {
            self.finish(Outcome::Negative);
        }
    }
}

pub fn reject(namespace: &str, pair_id: i32, height: Option<i64>, observed_at: DateTime<Utc>) {
    let mut reg = registry().lock().unwrap();
    prune(&mut reg);
    let owner = next(&mut reg);
    let mut invalidate = true;
    if let Some(ns) = reg.namespaces.get_mut(namespace) {
        if let Some(entry) = ns.entries.get_mut(&pair_id) {
            let rejected = Stamp {
                height,
                at_us: observed_at.timestamp_micros(),
            };
            let covers = |old: Stamp| {
                rejected.at_us >= old.at_us && (height.is_none() || rejected.covers(old))
            };
            if entry.validated.is_none_or(covers) && entry.watermark.is_none_or(covers) {
                entry.negative = true;
                entry.complete = None;
                entry.pool_complete = None;
                entry.phase = None;
                entry.owner = owner;
                entry.watermark = Some(rejected);
                ns.revision = ns.revision.saturating_add(1);
            } else {
                invalidate = false;
            }
        }
    }
    drop(reg);
    if invalidate {
        crate::api::local_quote::invalidate_fee_generation(namespace, pair_id);
    }
}
pub(crate) fn revision(namespace: &str) -> u64 {
    let mut reg = registry().lock().unwrap();
    prune(&mut reg);
    reg.namespaces.get(namespace).map_or(0, |ns| ns.revision)
}
pub(crate) fn permits(
    namespace: &str,
    pair_id: i32,
    height: Option<i64>,
    observed_at: DateTime<Utc>,
) -> bool {
    permits_view(namespace, pair_id, height, observed_at, false)
}
pub(crate) fn permits_view(
    namespace: &str,
    pair_id: i32,
    height: Option<i64>,
    observed_at: DateTime<Utc>,
    pool_view: bool,
) -> bool {
    let mut reg = registry().lock().unwrap();
    prune(&mut reg);
    reg.namespaces
        .get(namespace)
        .and_then(|ns| ns.entries.get(&pair_id))
        .is_none_or(|entry| {
            let incoming = Stamp {
                height,
                at_us: observed_at.timestamp_micros(),
            };
            let published = if pool_view {
                entry.pool_published
            } else {
                entry.published
            };
            !entry.negative
                && published.is_none_or(|published| incoming.covers(published))
                && entry
                    .watermark
                    .is_none_or(|watermark| incoming.covers(watermark))
        })
}
/// Exact pending/transient raw generation only. The returned tuple is the old
/// complete tuple itself, with its original static deadline; never splice fees.
#[cfg(test)]
pub(crate) fn retained(
    namespace: &str,
    pair_id: i32,
    context: &str,
    height: Option<i64>,
    observed_at: DateTime<Utc>,
    reserves: (u128, u128),
) -> Option<(Arc<HopMirror>, i64)> {
    let views = retained_views(namespace, pair_id, context, height, observed_at, reserves)?;
    views.hybrid.or(views.pool)
}
pub(crate) struct RetainedViews {
    pub hybrid: Option<(Arc<HopMirror>, i64)>,
    pub pool: Option<(Arc<HopMirror>, i64)>,
}
pub(crate) fn retained_views(
    namespace: &str,
    pair_id: i32,
    context: &str,
    height: Option<i64>,
    observed_at: DateTime<Utc>,
    reserves: (u128, u128),
) -> Option<RetainedViews> {
    let mut reg = registry().lock().unwrap();
    prune(&mut reg);
    let entry = reg.namespaces.get(namespace)?.entries.get(&pair_id)?;
    let phase = entry.phase?;
    if !entry.qualified
        || entry.negative
        || entry.context != context
        || phase.until <= Instant::now()
        || phase.stamp
            != (Stamp {
                height,
                at_us: observed_at.timestamp_micros(),
            })
        || phase.reserves != reserves
    {
        return None;
    }
    let now = Utc::now().timestamp_millis();
    let retain = |complete: &Option<Complete>| {
        let complete = complete.as_ref()?;
        (complete.expiry_ms.min(entry.expiry_ms) > now
            && !crate::api::db_orderbook_sim::is_snapshot_stale(
                complete.mirror.snapshot_at,
                complete.max_age_ms,
            )
            && phase.stamp.covers(stamp(&complete.mirror)))
        .then(|| {
            (
                complete.mirror.clone(),
                complete.expiry_ms.min(entry.expiry_ms),
            )
        })
    };
    let views = RetainedViews {
        hybrid: retain(&entry.complete),
        pool: retain(&entry.pool_complete),
    };
    (views.hybrid.is_some() || views.pool.is_some()).then_some(views)
}

#[cfg(test)]
#[path = "fee_phase_tests.rs"]
mod tests;

/// Release only an owned, completed positive unit fixture; unavailable in production.
#[cfg(test)]
pub(crate) fn retire_completed_test_namespace(namespace: &str) {
    let mut reg = registry().lock().unwrap();
    if let Some(owned) = reg.namespaces.get(namespace) {
        assert!(
            owned
                .entries
                .values()
                .all(|entry| entry.phase.is_none() && !entry.negative),
            "cannot retire active or negative fee evidence"
        );
    }
    reg.namespaces.remove(namespace);
}
