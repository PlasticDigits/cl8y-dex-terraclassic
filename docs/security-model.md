# Security Model

## Governance Keys

The Factory contract has a single `governance` address that controls:
- Adding/removing whitelisted CW20 code IDs
- Setting per-pair fee rates
- Registering post-swap hooks on pairs
- Updating the governance address itself, the treasury, and default fee

**Key management:** the governance address should be a multisig or DAO-controlled address in production. Never use a single EOA for mainnet governance.

Operator checklist (governance, treasury, hooks, router trust, pool-only verification): [`docs/runbooks/launch-checklist.md`](runbooks/launch-checklist.md).

## Treasury Management

All swap commissions are sent directly to the `treasury` address configured in the Factory. The Pair contract holds no fees — they are transferred atomically during each swap.

- The treasury address can be updated via `UpdateConfig` (governance only).
- Fee rate is denominated in basis points (1 bps = 0.01%). Max is 10000 (100%).
- Each pair can have an individually configured fee rate via `SetPairFee`.

## Code ID Whitelist

The Factory maintains a whitelist of CW20 code IDs. When `CreatePair` is called with `asset_infos`, both assets must be `AssetInfo::Token` (native tokens are rejected), and both token contract addresses are checked against this whitelist by querying each token's contract info on-chain.

**Rationale:** this prevents pairs from being created with malicious CW20 contracts that could manipulate balances, re-enter, or steal funds.

## Native Token Rejection

The `AssetInfo` enum includes a `NativeToken` variant for TerraSwap wire compatibility, but all contracts reject it at runtime with a clear error message. This prevents accidental use of native tokens until CW20 wrapping support is added.

## Hook Safety

Hooks are external contracts invoked via `AfterSwap` after every swap completes. The hook receives `offer_asset`, `return_asset`, `commission_amount`, and `spread_amount` as `Asset` structs. Risks and mitigations:

| Risk                    | Mitigation                                              |
|-------------------------|---------------------------------------------------------|
| Hook reverts -> swap fails| By design: hooks are not `reply_on_error`, so a reverting hook blocks the swap. Only register trusted hooks. |
| Reentrancy              | CosmWasm's actor model prevents cross-contract reentrancy within a single transaction. |
| Gas griefing             | Hooks consume gas from the swap caller. Only register hooks with bounded execution cost. |
| Data integrity           | Hook receives read-only data (amounts, addresses). It cannot modify pair state. |

**Best practice:** only governance should register hooks (enforced by the Factory auth check), and hooks should be audited before registration.

## Fee Discount Security

### EOA-Only Self-Registration

The `Register` message enforces that only externally owned accounts (EOAs) can self-register for discount tiers. The contract checks that `info.sender` is not a contract address (no code hash on-chain). This prevents smart contracts from gaming the discount system by programmatically registering and routing swaps through a registered wrapper.

Governance can bypass this restriction using `RegisterWallet` to register contracts explicitly — intended for whitelisted market maker contracts that operate at Tier 0.

### Governance-Only Tiers

Tier 0 (100% discount) and Tier 255 (blacklist / 0% discount) cannot be self-registered. They are reserved for governance actions:

- **Tier 0:** assigned to market maker contracts via `RegisterWallet` to grant zero-fee trading.
- **Tier 255:** assigned to wallets that should receive no discount (blacklist). A wallet registered at Tier 255 effectively gets the full pair fee on every swap.

### Trusted Routers

The fee-discount contract maintains a list of trusted routers. When the Pair receives a swap with a `trader` field, it only uses that field for discount lookup if the CW20 `Send` originated from a trusted router. This prevents an attacker from constructing a `Swap` message with an arbitrary `trader` address to steal someone else's discount.

Only governance can add or remove trusted routers via `AddTrustedRouter` / `RemoveTrustedRouter`.

### Balance Verification and Lazy Deregistration

The `GetDiscount` query checks the trader's CL8Y token balance against their registered tier's `min_tokens` threshold on every swap. If the balance is insufficient:

1. The contract returns `discount_bps: 0` for the current swap (no discount applied).
2. A fire-and-forget deregistration message is dispatched to remove the stale registration.

This lazy approach avoids the need for a background process or cron job to monitor balances. Traders who sell their CL8Y tokens lose their discount on the next swap automatically.

### Fee Discount Auth Summary

| Action                 | Authorized Caller     |
|------------------------|-----------------------|
| `AddTier`              | Governance            |
| `UpdateTier`           | Governance            |
| `RemoveTier`           | Governance            |
| `Register`             | EOA only (self)       |
| `RegisterWallet`       | Governance            |
| `Deregister`           | Self                  |
| `DeregisterWallet`     | Governance            |
| `AddTrustedRouter`     | Governance            |
| `RemoveTrustedRouter`  | Governance            |
| `UpdateConfig`         | Governance            |

## Pair Contract Auth

| Action              | Authorized Caller       |
|---------------------|-------------------------|
| Swap (CW20 Send)    | Any CW20 token          |
| ProvideLiquidity     | Anyone                  |
| WithdrawLiquidity    | LP token (via CW20 Send)|
| UpdateFee            | Factory only             |
| UpdateHooks          | Factory only             |
| SetDiscountRegistry  | Factory only             |

## Audit Status

Contracts have not yet been formally audited. A third-party audit is recommended before mainnet deployment with significant TVL.

For an **in-repo** invariant matrix, trust assumptions, and mapping to automated tests, see [contracts-security-audit.md](./contracts-security-audit.md).
