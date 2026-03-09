# Security Model

## Governance Keys

The Factory contract has a single `governance` address that controls:
- Adding/removing whitelisted CW20 code IDs
- Setting per-pair fee rates
- Registering post-swap hooks on pairs
- Updating the governance address itself, the treasury, and default fee

**Key management:** the governance address should be a multisig or DAO-controlled address in production. Never use a single EOA for mainnet governance.

## Treasury Management

All swap fees are sent directly to the `treasury` address configured in the Factory. The Pair contract holds no fees — they are transferred atomically during each swap.

- The treasury address can be updated via `UpdateConfig` (governance only).
- Fee rate is denominated in basis points (1 bps = 0.01%). Max is 10000 (100%).
- Each pair can have an individually configured fee rate via `SetPairFee`.

## Code ID Whitelist

The Factory maintains a whitelist of CW20 code IDs. When `CreatePair` is called, both token addresses are checked against this whitelist by querying each token's contract info on-chain.

**Rationale:** this prevents pairs from being created with malicious CW20 contracts that could manipulate balances, re-enter, or steal funds.

## Hook Safety

Hooks are external contracts invoked via `AfterSwap` after every swap completes. Risks and mitigations:

| Risk                    | Mitigation                                              |
|-------------------------|---------------------------------------------------------|
| Hook reverts → swap fails| By design: hooks are not `reply_on_error`, so a reverting hook blocks the swap. Only register trusted hooks. |
| Reentrancy              | CosmWasm's actor model prevents cross-contract reentrancy within a single transaction. |
| Gas griefing             | Hooks consume gas from the swap caller. Only register hooks with bounded execution cost. |
| Data integrity           | Hook receives read-only data (amounts, addresses). It cannot modify pair state. |

**Best practice:** only governance should register hooks (enforced by the Factory auth check), and hooks should be audited before registration.

## Pair Contract Auth

| Action          | Authorized Caller |
|-----------------|-------------------|
| Swap            | Any CW20 token (via Send) |
| AddLiquidity    | Anyone            |
| RemoveLiquidity | LP token (via Send) |
| UpdateFee       | Factory only      |
| UpdateHooks     | Factory only      |

## Audit Status

Contracts have not yet been formally audited. A third-party audit is recommended before mainnet deployment with significant TVL.
