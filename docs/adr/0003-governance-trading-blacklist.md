# ADR 0003: Governance-controlled trading blacklist

**Status:** Accepted  
**Issue:** [GitLab #308](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/308)

## Context

Tier 255 on the fee-discount contract removes fee discounts but does **not** stop trading. Compliance and incident response need governance to halt swaps, liquidity changes, limit placement/cancel/claim, and router multihop when a wallet, token, or pair is implicated.

## Decision

Store blacklist state on the **factory** (central registry):

| Dimension | Storage | Effect |
|-----------|---------|--------|
| Wallet | `BLACKLISTED_WALLETS` | Blocks actor on all pairs |
| Token | `BLACKLISTED_TOKENS` | Blocks any trade touching the CW20 |
| Pair | `BLACKLISTED_PAIRS` | Blocks all actions on that pair contract |

Governance-only execute messages: `BlacklistWallet`, `UnblacklistWallet`, `BlacklistToken`, `UnblacklistToken`, `BlacklistPair`, `UnblacklistPair`.

**Guards:** Pair and router call factory `BlacklistCheck` before user-facing executes. Pair also checks the optional `trader` field on router-originated swaps.

**Recovery:** Governance `Unblacklist*` restores service; user funds in escrow or LP are not destroyed—only protocol paths are gated.

## Consequences

- Deployed factories migrate to `1.5.0` with empty maps (no behavior change until governance acts).
- Pairs upgrade to `1.8.0` for guard wiring; router includes multihop `pairs` probe.
- dApp and indexer expose read-only `BlacklistCheck` for UX (LCD or `GET /api/v1/compliance/blacklist-check`).

## Alternatives considered

- Per-pair maps: rejected (N-way drift).
- Fee-discount tier only: insufficient (trading continues).
