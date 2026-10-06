# Token-owned AutoLP policy recovery (#1429)

Use for legacy sister inventory, explicit policy recovery or migration readback.
Related: [#1429](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1429),
[NE1398-9](AGENTS_NET_EXECUTION_FLOORS.md),
[AutoLP](AGENTS_COMMUNITY_TAX_AUTOLP.md),
[agent checklist and skip policy](../docs/qa/issue-1429/README.md).

| ID | Invariant |
| --- | --- |
| LP1429-1 | Inventory actual bound sister `GetConfig.manager` and `GetLpPolicy` before recovery. Null policy is distinct from unsupported query. Code-list inventory is bounded endpoint evidence, not proof of all chain deployments. |
| LP1429-2 | Token `RefreshAutolpPolicy` authenticates the configured token manager, requires AutoV2Lp and the configured binding. No caller-selected sister, generic dispatcher, governance impersonation or permissionless refresh. |
| LP1429-3 | The bound sister must identify this token as both token and manager, use the token's factory, retain the approved pair/quote and be unlocked. Human-managed sisters use their actual manager directly. |
| LP1429-4 | Caller approves exact pair, quote, LP token, positive tax/quote reserves and total shares. Live pool values must match exactly; changed/diluted/empty pools require fresh explicit approval. Unsupported/malformed policy queries fail closed. |
| LP1429-5 | Recovery emits only same-pair `UpdateConfig`; sister factory validation, manager authorization and registration still execute. Threshold, recipient, router and finite spread/return floors are preserved. Permissionless skim never captures a new anchor. |
| LP1429-6 | Identical paid settings remain a no-op with no fee kept and no policy write. Recovery is a separate nonpayable maintenance execute, with no UST1 invoice, SKU purchase, skim or change of economic settings. |
| LP1429-7 | Registration failure rolls back anchor changes. Subsequent swap/provide shortage and reply failure revert balances, pool, LP supply, allowances, pending state and lock. Never catch errors and report partial success. |
| LP1429-8 | Token 1.4.0 adds the proxy without storage migration; AutoLP policy wire/storage stays compatible with reviewed 1.1.0. Migration/code/version receipts and exact policy readback are separate from source verification. Production rollout and max-hook gas remain #1421. |

Run `make verify-issue-1429` and `make verify-issue-1398`; retest corrections.
Agents may record unavailable checks as SKIP and overall PASS WITH SKIPS when all
executed checks pass. Leave omitted acceptance rows unchecked and record observed
reasons; executed failures remain FAIL until fixed and retested. Source passes do
not establish deployment. See [recovery instructions](../docs/qa/issue-1429/README.md).

Sources: [proxy](../smartcontracts/contracts/community-tax-token/src/autolp_recovery.rs),
[shared policy](../smartcontracts/packages/dex-common/src/autolp.rs),
[sister approval](../smartcontracts/contracts/community-tax-autolp/src/contract.rs),
[economic floors](../smartcontracts/contracts/community-tax-autolp/src/lp_guard.rs).
