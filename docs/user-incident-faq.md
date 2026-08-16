# What happens during an incident?

Plain-language guide for **traders and liquidity providers** when CL8Y DEX governance or operators apply emergency controls: **pair pause**, **trading blacklist**, or **rate limits**.

This is **not** financial or legal advice. For operator runbooks and on-chain invariants, see [security-model.md](./security-model.md) and [contracts-security-audit.md](./contracts-security-audit.md).

How to **add or withdraw LUNC LP** on the dApp (not an incident): [user-lunc-liquidity.md](./user-lunc-liquidity.md) and `/pool#lp-howto` ([#531](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/531)).

## Quick summary

| Control | Who it affects | Are my funds safe? | What to do |
|---------|----------------|-------------------|------------|
| **Pair pause** | Everyone using that pool | Yes — tokens stay in the pair contract | Wait for unpause, then retry swaps / limit actions |
| **Wallet blacklist** | One address | Yes — balances are not seized | Wait for `UnblacklistWallet`, then retry |
| **Token blacklist** | Any trade touching that CW20 | Yes — wallet balances unchanged | Trade other tokens/pairs, or wait for lift |
| **Pair blacklist** | That pool only | Yes — LP and escrow stay on-chain | Use other pairs, or wait for lift |
| **Rate limit** | Whoever hit the cap | Yes — no on-chain movement blocked | Wait and retry (see below) |
| **UST1 window / oracle** | `/ust1` mint & redeem | Yes — tokens stay in your wallet | Wait for fresh oracle or unpause; retry |
| **Wrap pause** | Native wrap / unwrap | Yes — native & cLUNC/cUSTC stay in your wallet | Wait for operator unpause |

**Important:** These controls **block new protocol actions**. They do **not** burn, confiscate, or delete your wallet balances. Escrow for limit orders and LP shares remain in the contracts until you can execute the normal withdraw / cancel / claim messages again.

---

## Pair pause

Governance can pause a **single trading pair** (for example during an exploit investigation or oracle incident).

### What you will see

- The dApp shows a **pause banner** on Trade, Limit Orders, **Swap**, and **Pool** for that pair.
- Swap, add liquidity, new limit orders, cancel, claim parked expired limits, and limit price edits are **disabled** in the UI.
- On-chain transactions for those actions are **rejected** with a paused error.

### What is blocked

While a pair is paused ([invariant **L6**](./contracts-security-audit.md)):

- **Swaps** (pool and hybrid / limit-book fills)
- **Provide liquidity** (add to the pool)
- **Withdraw liquidity** (remove LP via the pair’s CW20 receive path)
- **Place** new limit orders
- **Cancel** resting limit orders
- **Claim** parked expired limit refunds
- **Update** limit order price
- **Clean** the limit book (permissionless maintenance)

### What happens to your assets

| Asset type | During pause | After unpause |
|------------|--------------|---------------|
| **Wallet CW20** | Unchanged in your wallet | Unchanged |
| **LP tokens** | Still in your wallet; pool share unchanged | Withdraw liquidity when unpaused |
| **Limit order escrow** | Held safely in the pair contract | Cancel resting orders or claim parked expired rows |
| **Parked expired limits** | Rows stay in `EXPIRED_LIMIT_CLAIMS` until you claim | Submit **Claim refund** (or batch claim) |

No automatic sweep of user funds occurs on pause. Governance **unpause** restores normal trading and withdrawal paths.

### Recovery path

1. Wait for governance to **unpause** the pair (`SetPairPaused { paused: false }`).
2. Refresh the dApp — the pause banner should clear.
3. Retry the action you attempted (swap, add/remove liquidity, place/cancel/claim limits).

If a transaction still fails, check [wallet](#wallet-blacklist) or [token/pair blacklist](#token-blacklist) below.

---

## Wallet blacklist

Governance can add a **wallet address** to the factory trading blacklist for compliance or incident response ([ADR 0003](./adr/0003-governance-trading-blacklist.md)).

### What you will see

- The dApp disables trade, pool, and limit buttons when your connected wallet is blocked.
- Message along the lines of: *“This wallet is on the protocol trading blacklist… Swaps, liquidity, and limit orders are disabled until governance removes the restriction.”*

### What is blocked

For the blacklisted address:

- Swaps (direct and via router)
- Provide and withdraw liquidity
- Limit order place, cancel, claim, and price update
- Router multihop that includes your address as trader
- **Resting limit fills** — if you already had orders on the book when blacklisted, takers cannot fill them; escrow parks for claim after unblacklist ([#468](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/468))

### What happens to your assets

- **CW20 and LP tokens** remain in your wallet.
- **Limit escrow** and **LP pool shares** are not destroyed; you simply cannot execute protocol messages until the restriction lifts.
- This is **not** the same as fee-discount **tier 255** (that only removes fee discounts; trading continues).

### Recovery path

1. Wait for governance **`UnblacklistWallet`**.
2. Reconnect the wallet and retry your action.
3. If you believe the listing was in error, contact the protocol operator through official channels (not via on-chain messages).

---

## Token blacklist

Governance can blacklist a **CW20 token**. Any pair that includes that token is affected in **both directions**.

### What you will see

- Trading involving the token is disabled (swap, liquidity, limits on affected pairs).
- Unaffected pairs (tokens not on the list) continue to work for non-blacklisted wallets.

### What happens to your assets

- Token balances in your wallet are **unchanged**.
- LP positions in affected pairs remain on-chain; you recover them after the token is unblacklisted and you can withdraw.

### Recovery path

1. Wait for **`UnblacklistToken`**, or trade only on pairs that do not include the token.
2. After lift, retry withdraw / cancel / claim as needed.

---

## Pair blacklist

Governance can blacklist a **specific pair contract**. Only that pool is gated; other pairs stay open.

### What you will see

- Same class of blocks as pause for that pool (swaps, liquidity, limits).
- Other pools are unaffected.

### What happens to your assets

Same as [pair pause](#what-happens-to-your-assets-1): LP shares and limit escrow stay in the pair contract until you can withdraw or claim after **`UnblacklistPair`**.

### Recovery path

1. Wait for **`UnblacklistPair`**.
2. Retry liquidity withdrawal and limit cancel/claim.

---

## Rate limits

Rate limits protect infrastructure. They are **not** a seizure of funds.

### Indexer / API (quotes, charts, order book)

The public indexer limits requests per IP (default **60/s** globally, **10/s** on LCD-heavy routes such as route solve and deep order books). Sustained abuse returns **HTTP 429** with `Retry-After` and `x-ratelimit-*` headers.

| Symptom | Impact | What to do |
|---------|--------|------------|
| Quote or chart fails to load | Off-chain display only; your wallet is unaffected | Wait for `Retry-After`, reduce refresh frequency, retry |
| Route solve 429 | dApp may not show a route until retry succeeds | Wait and retry; swap confirmation still uses on-chain checks |

No tokens move on a 429 — it is an HTTP throttle only.

### Native wrap / unwrap (wrap-mapper)

**Unwrap receive vs fee ([#512](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/512)):** Unwrapping pays the wrap-mapper fee **and** Classic burn tax on the treasury’s native payout. The dApp **You Receive** figure includes both. Prefer withdrawing to **your own wallet** — exchange deposit addresses often ignore contract-initiated transfers even with a memo.

Governance may configure **per-denom wrap limits** (maximum native amount per time window). Large wrap or instant-withdraw attempts above the cap fail with a rate-limit error **before** completing.

| Symptom | Impact | What to do |
|---------|--------|------------|
| Wrap fails “rate limit” | No wrap completed; native stays in your wallet | Wait for the window to reset, retry a smaller amount |
| Treasury wrapping paused | Wrap and instant withdraw disabled | Wait for operator to unpause wrapping |

See [NATIVE_TOKEN_WRAPPING.md](../NATIVE_TOKEN_WRAPPING.md) for wrap architecture. This is **not** the same as indexer **HTTP 429** (off-chain quotes/charts).

---

## UST1 oracle window

The **`/ust1`** page mints and redeems **UST1 ↔ vFDUSD** through the on-chain oracle window. It is **not** the soft-launch faucet (**Mint**) and **not** an AMM swap.

### What you will see

- Deposit / withdraw disabled when the window is paused, the oracle is paused, or the oracle rate is **stale** (too old).
- Clear CTA copy such as window paused / oracle unavailable — do not force-submit.

### What happens to your assets

| Situation | Impact | What to do |
|-----------|--------|------------|
| Window paused | No mint/redeem; balances unchanged | Wait for operators to unpause |
| Oracle paused or oracle stale | Quotes/gates block submit; funds stay in wallet | Wait for a fresh oracle update; retry later |
| Per-tx or 24h limit | Transaction rejected before completion | Use a smaller amount or wait for the rolling window |

Operators restore the oracle service — they do **not** turn off age checks to fake a healthy rate. See [`ust1-wrap-production-ops.md`](./runbooks/ust1-wrap-production-ops.md).

---

## Wrap pause

Governance can pause the **wrap-mapper** so native **LUNC/USTC ↔ cLUNC/cUSTC** wrap and unwrap stop. Ordinary CW20 pool swaps that do not wrap/unwrap can still work.

### What you will see

- Swap / Wrap UI: **Wrapping is Temporarily Paused** (or equivalent).
- On-chain wrap and unwrap reject with a paused error.

### What happens to your assets

| Asset | During wrap pause | After unpause |
|-------|-------------------|---------------|
| Native LUNC / USTC in wallet | Unchanged | Wrap again when unpaused |
| cLUNC / cUSTC in wallet | Unchanged | Unwrap when unpaused |
| LP / other CW20 | Unaffected by wrap-mapper pause | Trade as usual |

Operator playbook: [`wrap-mapper-pause.md`](./runbooks/wrap-mapper-pause.md).

---

## Pair creation (factory)

`CreatePair` is limited to **one new pair flow per block** and may charge a creation fee. This affects **pool creators**, not everyday swappers. Retry on the next block if creation fails.

---

## How the dApp helps

| Signal | UI behavior |
|--------|-------------|
| **Pause** | `IsPaused` query drives banners on Trade, Limit Orders, Swap, and Pool. |
| **Blacklist** | Factory `BlacklistCheck` (and indexer `GET /api/v1/compliance/blacklist-check`) disables actions before you sign. |
| **Errors** | Failed transactions are humanized where possible (paused / blacklist messages). |

For technical integration details, see [integrators.md](./integrators.md) and [limit-orders.md](./limit-orders.md) (pause section).

---

## Related documentation

| Audience | Document |
|----------|----------|
| **Users (this page)** | [user-incident-faq.md](./user-incident-faq.md) |
| **Security overview** | [security-model.md](./security-model.md) |
| **On-chain invariants** | [contracts-security-audit.md](./contracts-security-audit.md) (e.g. **L6** pause) |
| **Blacklist design** | [adr/0003-governance-trading-blacklist.md](./adr/0003-governance-trading-blacklist.md) |
| **Operators** | [runbooks/](./runbooks/), [templates/incident-dex-indexer.md](./templates/incident-dex-indexer.md) (incident tracker + [Communications templates](./templates/incident-dex-indexer.md#appendix-communications-templates-sec-g05), SEC-G05) |

**Agent playbook:** [`skills/AGENTS_USER_INCIDENT_FAQ.md`](../skills/AGENTS_USER_INCIDENT_FAQ.md).
