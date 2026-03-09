# TWAP Oracle

Geometric-mean Time-Weighted Average Price oracle built into every CL8Y DEX pair contract.

## How It Works

Every pair contract maintains a **ring buffer of observations**. On each
state-changing action (swap, provide liquidity, withdraw liquidity), the
contract records a snapshot *before* mutating reserves:

```
tick = log₂(reserve_b / reserve_a)          (Q64.64 fixed-point)
tick_cumulative += tick × (block_time − last_observation_time)
```

The cumulative tick is the discrete integral of log₂(price) over time.
Consumers query two cumulative tick values separated by a time window and
derive the geometric-mean TWAP:

```
avg_tick = (tick_cumulative_end − tick_cumulative_start) / time_elapsed
price    = 2^(avg_tick / 2^64)
```

The geometric mean is used instead of the arithmetic mean because it is
**strictly harder to manipulate** — an attacker cannot spike the price in a
single block and disproportionately skew the average the way they can with
an arithmetic mean.

## Contract Interface

### Execute Messages

#### `IncreaseObservationCardinality`

Grow the observation ring buffer. Anyone can call this (caller pays gas).
The default cardinality is **360** (~36 minutes of history at 6-second
blocks). The maximum is **65 535** (~109 hours).

```json
{
  "increase_observation_cardinality": {
    "new_cardinality": 3600
  }
}
```

### Query Messages

#### `Observe`

Returns cumulative tick values at the requested `seconds_ago` offsets
(relative to the current block time). Pass at least two points to compute
a TWAP.

```json
{
  "observe": {
    "seconds_ago": [0, 1800]
  }
}
```

Response:

```json
{
  "tick_cumulatives": [<i128>, <i128>]
}
```

To compute the TWAP price from the response, use the
`dex_common::oracle::compute_twap_price` helper (Rust) or perform the
calculation off-chain:

```python
avg_tick = (tick_cumulatives[0] - tick_cumulatives[1]) / (1800 - 0)
price = 2 ** (avg_tick / 2**64)
```

#### `OracleInfo`

Returns metadata about the oracle ring buffer:

```json
{
  "oracle_info": {}
}
```

Response:

```json
{
  "observation_cardinality": 360,
  "observation_index": 42,
  "oldest_observation_timestamp": 1700000000,
  "newest_observation_timestamp": 1700002100
}
```

## Choosing a TWAP Window

| Window | Manipulation Cost | Freshness | Recommended For |
|--------|-------------------|-----------|-----------------|
| 5 min  | Low               | High      | UI display only |
| 30 min | Moderate          | Moderate  | Lending / money markets with additional safeguards |
| 1 hr   | Good              | Low       | Perp mark prices with deviation checks |
| 24 hr  | Very high         | Very low  | Reference pricing, governance |

Manipulation cost scales **linearly** with both the window duration and the
pool's liquidity depth.

## Security Considerations

### Risks

1. **Multi-block manipulation.** A validator (or colluding validator set)
   that proposes consecutive blocks can trade at an artificial price across
   those blocks to shift the TWAP. On Terra Classic's Tendermint consensus,
   proposer rotation limits but does not eliminate this risk. Short windows
   (< 10 minutes) are especially vulnerable.

2. **Low-liquidity pools.** The cost to move a pool's spot price is
   proportional to its reserves. A $50k TVL pool can be moved 10% for
   roughly $5k of capital (recovered via arbitrage on another venue), which
   is cheap enough to exploit a lending protocol. **Do not rely on this TWAP
   for pools with < $100k TVL without additional safeguards.**

3. **Observation staleness.** If no user interacts with a pair for an
   extended period, no new observations are written. The `Observe` query
   linearly interpolates between the last known observation and the current
   block using the current reserves, but the accuracy degrades. High-value
   consumers should run a **keeper** that pokes the pair periodically.

4. **Geometric-mean bias.** The geometric mean is always ≤ the arithmetic
   mean. For volatile pairs this underestimate can be material. This is a
   feature for manipulation resistance but consumers should be aware of it.

5. **Single-source dependency.** This oracle derives from the pair's own
   reserves. If the pair is itself subject to an exploit (e.g. a bug in
   the swap math), the oracle is compromised too.

### Recommendations for Perps and Money Markets

This TWAP is designed as a **primary** price source, but high-value
protocols should **never** rely on it alone. Defense-in-depth means
layering multiple independent checks:

1. **Secondary oracle feed.** Cross-validate the TWAP against an
   independent price source — Band Protocol, an off-chain relay signed by
   trusted operators, or a governance-set reference price. Reject prices
   that deviate by more than a configurable threshold (e.g. 5%).

2. **Circuit breaker.** If the TWAP deviates from the secondary feed (or
   from the pair's own spot price) by more than N%, pause liquidations and
   new borrows until the deviation resolves. This limits damage from both
   oracle manipulation and genuine market dislocations.

3. **Minimum-liquidity gate.** Disable the oracle (or flag it as untrusted)
   for pairs whose TVL falls below a governance-defined floor. This
   prevents cheap manipulation of tail assets from cascading into
   liquidation events.

4. **Staleness check.** Reject observations older than a configurable
   maximum age. If the newest observation is too old, either pause or fall
   back to the secondary feed.

5. **Rate-of-change cap.** Reject TWAP prices that have moved more than a
   maximum percentage per unit time. Genuine markets rarely move 20% in 30
   minutes; if the TWAP claims that happened, treat it as suspect.

### Example: Hardened Oracle Consumer (Pseudocode)

```rust
fn get_safe_price(pair: Addr, window: u32, band_feed: Addr) -> Result<Decimal> {
    let twap = query_observe(pair, vec![0, window])?;
    let twap_price = compute_twap_price(twap[0], twap[1], window)?;

    let band_price = query_band(band_feed)?;

    let deviation = (twap_price - band_price).abs() / band_price;
    if deviation > Decimal::percent(5) {
        return Err("TWAP/Band deviation exceeds 5% — circuit breaker");
    }

    let oracle_info = query_oracle_info(pair)?;
    let age = env.block.time - oracle_info.newest_observation_timestamp;
    if age > MAX_STALENESS {
        return Err("TWAP observation too stale");
    }

    Ok(twap_price)
}
```

## Architecture Diagram

```
┌─────────────┐   swap / liquidity    ┌──────────────────┐
│   User /    │ ────────────────────► │   Pair Contract   │
│   Router    │                       │                   │
└─────────────┘                       │  ┌─────────────┐  │
                                      │  │  Reserves   │  │
                                      │  └──────┬──────┘  │
                                      │         │ sample   │
                                      │         ▼ BEFORE   │
                                      │  ┌─────────────┐  │
                                      │  │ Observation  │  │
                                      │  │ Ring Buffer  │  │
                                      │  └──────┬──────┘  │
                                      └─────────┼─────────┘
                                                │
                           ┌────────────────────┼───────────────────┐
                           │ Observe query      │                   │
                           ▼                    ▼                   ▼
                    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
                    │   Perps      │    │   Lending    │    │   Off-chain  │
                    │   Protocol   │    │   Protocol   │    │   Indexer    │
                    └──────┬───────┘    └──────┬───────┘    └──────────────┘
                           │                   │
                           ▼                   ▼
                    ┌─────────────────────────────────┐
                    │   Secondary Oracle (Band /      │
                    │   off-chain relay) for cross-   │
                    │   validation & circuit breaking  │
                    └─────────────────────────────────┘
```

## File Map

| File | Purpose |
|------|---------|
| `packages/dex-common/src/oracle.rs` | Core math (`log2_ratio_q64`, `exp2_tick_to_decimal`, `compute_twap_price`), observation types, response types |
| `contracts/pair/src/state.rs` | `OracleState`, `OBSERVATIONS` ring buffer storage |
| `contracts/pair/src/contract.rs` | `oracle_update` (hot path), `oracle_observe_single` (query), `IncreaseObservationCardinality` execute |
| `packages/dex-common/src/pair.rs` | `Observe` and `OracleInfo` query message definitions |
