# TWAP Oracle

Arithmetic-mean Time-Weighted Average Price oracle built into every CL8Y DEX pair contract.

The live pair stores **cumulative Decimal** of `reserve_b / reserve_a` (token1 **base units** per token0 **base unit**), not a geometric tick accumulator. Older copies of this page that described `log₂` ticks were stale ([GitLab **#564**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/564)). Canonical math: [`smartcontracts/packages/dex-common/src/oracle.rs`](../smartcontracts/packages/dex-common/src/oracle.rs). Charts display: [`docs/frontend.md` § Charts pair 24h Stats](./frontend.md#charts-pair-24h-stats).

## How It Works

Every pair contract maintains a **ring buffer of observations**. On each
state-changing action (swap, provide liquidity, withdraw liquidity), the
contract records a snapshot *before* mutating reserves:

```
price          = reserve_b / reserve_a          (CosmWasm Decimal, 18 digits)
price_a_cum   += price × dt                     (∫ token1_base / token0_base dt)
price_b_cum   += (reserve_a / reserve_b) × dt
```

Consumers query two cumulative values separated by a time window and
derive the **arithmetic-mean** TWAP:

```
twap_raw = (cum_end − cum_start) / time_elapsed
```

`twap_raw` is still **raw** token1 base units per token0 base unit — the same
units as on-chain limit `price` ([#529](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/529) **L529-1**). Human quote-per-base is:

```
twap_human = twap_raw × 10^(decimals0 − decimals1)
```

The dApp uses [`rawLimitPriceToHuman`](../frontend-dapp/src/utils/limitOrderPriceScale.ts) then `formatPairPrice`. Do **not** compact-format the raw Decimal as `T` ([#564](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/564)). TWAP is **not** USD; Charts **Price (USD)** stays factory `price_usd` / `invertUsd` ([#543](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/543)).

On-chain accumulation stays raw. Do not change CosmWasm units to “fix” the Charts display.

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

Returns cumulative **price** values at the requested `seconds_ago` offsets
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
  "price_a_cumulatives": ["<uint128>", "<uint128>"],
  "price_b_cumulatives": ["<uint128>", "<uint128>"]
}
```

To compute the TWAP price from the response, use the
`dex_common::oracle::compute_twap_price` helper (Rust) or
[`computeTwapPriceDecimalString`](../frontend-dapp/src/services/terraclassic/oracle.ts)
off-chain:

```python
# price_a_cumulatives[0] is "now"; [1] is seconds_ago[1]
twap_raw = (price_a_cumulatives[0] - price_a_cumulatives[1]) / 1800 / 1e18
twap_human = twap_raw * 10 ** (decimals0 - decimals1)
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

4. **Arithmetic-mean sensitivity.** This oracle is an **arithmetic** mean of
   raw `reserve_b / reserve_a`. A short spike still weights by time, but it
   is **not** the geometric-mean (Uniswap v3-style tick) construction. Short
   windows remain easier to skew than a geometric TWAP would be. Charts must
   not present the raw Decimal as a compact `T` figure ([#564](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/564)).

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
| `packages/dex-common/src/oracle.rs` | Arithmetic cumulative Decimal (`price_times_dt`, `compute_twap_price`), observation types, response types |
| `contracts/pair/src/state.rs` | `OracleState`, `OBSERVATIONS` ring buffer storage |
| `contracts/pair/src/contract.rs` | `oracle_update` (hot path), `oracle_observe_single` (query), `IncreaseObservationCardinality` execute |
| `packages/dex-common/src/pair.rs` | `Observe` and `OracleInfo` query message definitions |
| `frontend-dapp/src/services/terraclassic/oracle.ts` | LCD `observe` → raw Decimal string |
| `frontend-dapp/src/utils/chartsPairStats.ts` | Human TWAP display (`formatTwapHumanPrice`) |
