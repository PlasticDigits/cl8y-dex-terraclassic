# Security Model

Public **launch posture** (capped expectations, audit disclaimer, TVL-scaled controls): [`security-posture.md`](./security-posture.md) — linked from the dApp footer (SEC-A01, GitLab [#387](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/387)).

## User incident guide (pause, blacklist, rate limits)

Traders and liquidity providers: see **[What happens during an incident?](user-incident-faq.md)** for plain-language impact on swaps, LP positions, limit-order escrow, and recovery after governance lifts a restriction. Linked from the dApp legal footer ([GitLab **#390**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/390), SEC-A03). Agent playbook: [`skills/AGENTS_USER_INCIDENT_FAQ.md`](../skills/AGENTS_USER_INCIDENT_FAQ.md).

## Governance Keys

The Factory contract has a single `governance` address that controls:
- Adding/removing whitelisted CW20 code IDs
- Setting per-pair fee rates
- Registering post-swap hooks on pairs
- Updating the governance address itself, the treasury, and default fee

**Key management:** the governance address should be a multisig or DAO-controlled address in production. Never use a single EOA for mainnet governance.

Operator checklist (governance, treasury, hooks, router trust, pool-only verification): [`docs/runbooks/launch-checklist.md`](runbooks/launch-checklist.md). **Production mainnet** requires **Phase 5 go/no-go sign-off** on the launch tracking issue ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)); see [`skills/AGENTS_LAUNCH_GO_NO_GO.md`](../skills/AGENTS_LAUNCH_GO_NO_GO.md).

## Off-chain trust boundaries (frontend)

The on-chain contracts enforce swap math, whitelist, and auth — but the **browser dApp** also depends on off-chain services that are **not** cryptographically bound to chain state. Operators and users should understand these limits ([GitLab **#378**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378), parent [#376](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/376)).

| Surface | Trust assumption | Risk if compromised | Mitigation |
|---------|------------------|---------------------|------------|
| **Indexer** (`VITE_INDEXER_URL`) | Route solve (`router_operations`), token metadata (`logo_url`), charts, oracle display | Malicious but **valid** pool routes, inflated quotes, phishing logos | Pin **HTTPS-only** indexer URL in deploy; TLS cert from your CA; human-readable **Route** row at swap confirmation (`swap-route-summary`); optional factory LCD `getPair()` preflight on hops ([`swapRoutePreflight.ts`](../frontend-dapp/src/services/terraclassic/swapRoutePreflight.ts)). **No** client-side BFS fallback vs indexer (RPC rate limits). |
| **Build env** | `VITE_*` addresses, `VITE_WC_PROJECT_ID`, dev mnemonic | Wrong contracts inlined; shared WC project; seed in bundle | `vite build` guards: reject `VITE_DEV_MNEMONIC` outside `development` / `VITE_ALLOW_DEV_MNEMONIC=local-only`; require `VITE_WC_PROJECT_ID` in production. See [`docs/frontend.md`](./frontend.md#simulated-dev-wallet-and-vite_dev_mnemonic). |
| **Token logos** | Indexer `logo_url` + static registry | Phishing imagery for look-alike tickers | Host allowlist in [`TokenLogo`](../frontend-dapp/src/components/ui/TokenLogo.tsx); indexer token listing requires **human review** ([`docs/CG_CMC_COMPLIANCE.md`](./CG_CMC_COMPLIANCE.md#token-listing-review)). |
| **LCD / RPC** | `VITE_TERRA_LCD_URL`, `VITE_TERRA_RPC_URL` | MITM on queries/simulations | HTTPS endpoints; optional startup factory address sanity check documented in deploy checklist. |

**Deploy checklist (frontend):**

1. Set `VITE_INDEXER_URL` to your **HTTPS** indexer origin only (no mixed-content `http:` on public sites).
2. Pin `VITE_FACTORY_ADDRESS`, `VITE_ROUTER_ADDRESS`, and related contract env vars per network; verify on [`/protocol`](../frontend-dapp/src/pages/ProtocolPage.tsx) (audit surface only — not swap confirmation).
3. Set a dedicated `VITE_WC_PROJECT_ID` (WalletConnect Cloud) for production.
4. Production CSP `connect-src` lists env LCD/RPC/indexer hosts + WalletConnect relay — not blanket `https:` ([`frontend-dapp/viteCsp.ts`](../frontend-dapp/viteCsp.ts)).

TLS pinning guidance: use a managed TLS cert on the indexer load balancer; restrict DNS to operator-controlled records; monitor cert expiry. Full MITM resistance also depends on the user’s browser trust store — document honest limits in operator runbooks.

## Treasury Management

All swap commissions are sent directly to the `treasury` address configured in the Factory. The Pair contract holds no fees — they are transferred atomically during each swap.

- The treasury address can be updated via `UpdateConfig` (governance only).
- Fee rate is denominated in basis points (1 bps = 0.01%). Max is 10000 (100%).
- Each pair can have an individually configured fee rate via `SetPairFee`.

## Code ID Whitelist

The Factory maintains a whitelist of CW20 code IDs. When `CreatePair` is called with `asset_infos`, both assets must be `AssetInfo::Token` (native tokens are rejected), and both token contract addresses are checked against this whitelist by querying each token's contract info on-chain.

**Rationale:** this prevents pairs from being created with malicious CW20 contracts that could manipulate balances, re-enter, or steal funds.

**Fee-on-transfer prohibition (GitLab [#377](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/377)):** do **not** whitelist CW20 templates that skim on `transfer` / `send`. Pair reserves credit declared amounts, not balance deltas — fee-on-transfer tokens desync reserves (invariant **P2**). Ops runbook: [`docs/runbooks/cw20-whitelist-policy.md`](runbooks/cw20-whitelist-policy.md). Verify production GDEX/TerraPort code IDs via [`scripts/verify-cw20-code-ids.sh`](../scripts/verify-cw20-code-ids.sh) before `AddWhitelistedCodeId`.

## `CreatePair` rate limit and pending state

`CreatePair` is **permissionless** (any address may call it, subject to whitelist checks). The factory keeps a single `PENDING_PAIR` slot while a pair `Instantiate` submessage is in flight inside **that** transaction.

**Cosmos atomicity:** each transaction runs to completion—including submessages and `reply` handlers—before the next transaction runs on the same contract. A second wallet submitting `CreatePair` in the same block therefore **does not** interleave between the first call’s `execute` and `reply`; the cross-transaction `PENDING_PAIR` overwrite described in hypothetical race analysis does **not** apply on standard Terra Classic / Cosmos SDK execution.

**Per-block gate (product invariant):** the factory still records `PAIR_CREATION_BLOCK` and rejects a second `CreatePair` that would start another instantiate flow in the **same** block height. That limits pairing spam and encodes “one pending create path per block” for operators and automation. Scripts that create multiple pairs must use one pair per block (e.g. wait for the next block) or sequence heights explicitly.

**Pair-creation fee (GitLab [#276](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/276)):** `CreatePair` requires attached **uluna** ≥ `config.pair_creation_fee_uluna` (default **100 LUNC** = `100_000_000` uluna). The fee is `BankMsg::Send` to `config.treasury`; stray denoms are rejected and any overpay is refunded to the sender. The charge is atomic with pair instantiate — if instantiate fails, the tx reverts and no fee is taken. Governance can raise or lower the fee via `SetPairCreationFee`. This makes sustained one-per-block griefing costly (attacker pays the fee every block) while honest users retry on the next block. Local deploy ([`scripts/deploy-dex-local.sh`](../scripts/deploy-dex-local.sh)) instantiates the factory with the real default fee and attaches it on every `create_pair`, so the LocalTerra path exercises the same fee economics as mainnet (treasury is the deploy address, so the fee returns to the deployer); set `LOCAL_PAIR_CREATION_FEE_ULUNA=0` for a fee-free local chain (GitLab [#318](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/318)).

See also: GitLab [#121](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/121), [#276](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/276), [`docs/contracts-security-audit.md`](./contracts-security-audit.md) (invariants **F1**, **F2**), and factory `lib.rs` / `state.rs` in `smartcontracts/contracts/factory/`.

## Native Token Rejection

The `AssetInfo` enum includes a `NativeToken` variant for TerraSwap wire compatibility, but all contracts reject it at runtime with a clear error message. This prevents accidental use of native tokens until CW20 wrapping support is added.

## Hook Safety

Hooks are external contracts invoked via `AfterSwap` after every swap completes. The hook receives `offer_asset`, `return_asset`, `commission_amount`, and `spread_amount` as `Asset` structs. Risks and mitigations:

| Risk                    | Mitigation                                              |
|-------------------------|---------------------------------------------------------|
| Hook reverts -> swap fails| By design: hooks are not `reply_on_error`, so a reverting hook blocks the swap. Only register trusted hooks. Intentional blocking (AML/incident) is allowed — see [hook registration runbook](runbooks/hook-registration.md). |
| Reentrancy              | Cosmwasm's actor model prevents cross-contract reentrancy within a single transaction. |
| Gas griefing             | Hooks consume gas from the swap caller. Only register hooks with bounded execution cost. |
| Data integrity           | Hook receives read-only data (amounts, addresses). It cannot modify pair state. |
| LP-burn spoofing        | LP-burn hook requires `AfterSwap.pair == info.sender` and queries pair `liquidity_token` (**H-03**). Allowlist only real pair contracts. |
| Tax/burn treasury drain | Tax and burn hooks charge from swap ask-token settlement forwarded by the pair (**I-02**); pre-funded hook balances are not required for normal fees. |

**Best practice:** only governance should register hooks (enforced by the Factory auth check), and hooks should be audited before registration. Full playbook: [`docs/runbooks/hook-registration.md`](runbooks/hook-registration.md).

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

The `GetDiscount` query checks the trader's CL8Y token balance against their registered tier's `min_cl8y_balance` threshold on every swap. If the balance is insufficient:

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

## Trading blacklist (compliance / incident response)

Governance on the **factory** can block protocol interaction without bricking unrelated users' balances permanently. State lives in `BLACKLISTED_WALLETS`, `BLACKLISTED_TOKENS`, and `BLACKLISTED_PAIRS` (see [ADR 0003](adr/0003-governance-trading-blacklist.md), GitLab [#308](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/308)).

| Dimension | Blocks |
|-----------|--------|
| Wallet | Swaps, hybrid swaps, provide/withdraw liquidity, limit place/cancel/claim/update, router multihop for that address |
| Token | Any trade on any pair that includes the CW20 (both directions) |
| Pair | All user actions on that pair contract |

**Not the same as Tier 255:** fee-discount tier 255 only removes discounts; trading continues. Use factory blacklist when trading must halt.

**Recovery:** `UnblacklistWallet`, `UnblacklistToken`, or `UnblacklistPair` restores normal operation. Escrow and LP positions remain on-chain; only gated execute paths are rejected. User-facing explanation: [user-incident-faq.md](./user-incident-faq.md).

**Queries:** `BlacklistCheck { wallet, tokens, pair, pairs }` on the factory. Indexer proxy: `GET /api/v1/compliance/blacklist-check`. dApp disables CTAs when blocked.

**Frontend regression tests (SEC-A02, GitLab [#388](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/388)):** Vitest mocks `useTradingBlacklist` on Swap and Trade order ticket and asserts `describeTradingBlacklistBlock` copy in the alert plus disabled swap / limit-place CTAs for wallet, token, and pair dimensions. Copy source: [`blacklist.ts`](../frontend-dapp/src/services/terraclassic/blacklist.ts); shared mocks: [`tradingBlacklistMocks.ts`](../frontend-dapp/src/test/tradingBlacklistMocks.ts).

## Pair Contract Auth

| Action              | Authorized Caller       |
|---------------------|-------------------------|
| Swap (CW20 Send)    | Any CW20 token          |
| ProvideLiquidity     | Anyone                  |
| WithdrawLiquidity    | LP token (via CW20 Send)|
| UpdateFee            | Factory only             |
| UpdateHooks          | Factory only             |
| SetDiscountRegistry  | Factory only             |

## Off-chain trust boundaries (frontend / indexer)

On-chain contracts enforce swap safety (whitelist, max spread, slippage min-return, pause). **Off-chain services can still mislead users** into signing transactions that are valid on-chain but economically harmful. Remediation tracked in GitLab [#376](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/376) / [#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/378).

### Indexer route quotes (H-04)

The dApp **trusts the configured indexer** for multi-hop `router_operations` after JSON parsing and per-hop factory LCD `getPair()` preflight ([`swapRoutePreflight.ts`](../frontend-dapp/src/services/terraclassic/swapRoutePreflight.ts)). It does **not** re-derive routes client-side (RPC/LCD rate limits).

| Risk | Mitigation |
|------|------------|
| MITM on indexer HTTP | Deploy **`VITE_INDEXER_URL` over HTTPS only**; terminate TLS at your edge; pin or monitor cert changes. |
| Compromised indexer | Malicious but **valid** pools can appear in routes; user sees hop summary at confirmation (`data-testid="swap-route-summary"`). Operators must run a trusted indexer or accept quote risk. |
| Stale / wrong indexer env | Pin factory/router in build env; verify on [`/protocol`](../frontend-dapp/src/pages/ProtocolPage.tsx) (audit surface only). Optional LCD check: `VITE_VERIFY_DEPLOY_ADDRESSES=true` + [`deployAddressVerification.ts`](../frontend-dapp/src/utils/deployAddressVerification.ts). |

**Out of scope:** client-side BFS route fallback or on-chain hop graph cross-check in the browser.

### Build-time secrets and addresses

| Finding | Guard |
|---------|--------|
| Dev mnemonic inlined (H-05) | `vite build` fails when `VITE_DEV_MNEMONIC` is set outside `mode=development`, unless `VITE_ALLOW_DEV_MNEMONIC=local-only`. |
| Shared WalletConnect ID (M-10) | Production `vite build` requires `VITE_WC_PROJECT_ID`; no shared default in production bundles. |
| Wrong factory/router (M-08) | Set `VITE_FACTORY_ADDRESS` / `VITE_ROUTER_ADDRESS` per network; display on `/protocol` only. |

### Token metadata (M-09)

Indexer `logo_url` and symbols are **display hints**, not on-chain truth. Remote logos are allowlisted by hostname in [`tokenLogoAllowlist.ts`](../frontend-dapp/src/utils/tokenLogoAllowlist.ts); untrusted URLs fall back to blockies. **Indexer token listing** (`assets.logo_url`) should be updated only after **human review** — see [`operator-secrets.md`](./operator-secrets.md) and [`indexer-invariants.md`](./indexer-invariants.md).

### Expert mode (M-15)

Retail swaps block above **30%** expected route slippage unless Expert Mode is enabled (typed confirmation `ENABLE EXPERT MODE`). Settings slippage tolerance remains capped at **50%** for expert users. Thresholds unchanged — friction added at enable only.

### Content Security Policy (M-07)

Production builds emit a **narrow `connect-src`** (LCD, RPC, indexer, WalletConnect relays) — no blanket `https:`. Vite dev server keeps a broader policy for HMR. Bootstrap scripts live under `/bootstrap/*.js` so production `script-src` is `'self'` only. See [`docs/frontend.md` § Trust boundaries](./frontend.md#frontend-trust-boundaries).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TRUST_BOUNDARIES.md`](../skills/AGENTS_FRONTEND_TRUST_BOUNDARIES.md).

## User security contact (SEC-A07)

End users who see suspicious trades, unexpected balances, or misleading UI states must have a published escalation path ([GitLab **#392**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/392)):

| Surface | Location |
|---------|----------|
| Policy | [`SECURITY.md`](../SECURITY.md) at repo root — email, GitLab template, responsible disclosure, **48–72 hour** acknowledgement window |
| GitLab template | [`.gitlab/issue_templates/security_report.md`](../.gitlab/issue_templates/security_report.md) — `security` label, structured fields |
| dApp footer | [`LegalFooterNotice`](../frontend-dapp/src/components/legal/LegalFooterNotice.tsx) — **Report suspicious activity** → GitLab security template (`SECURITY_REPORT_ISSUE_URL` in [`legalCopy.ts`](../frontend-dapp/src/components/legal/legalCopy.ts)) |

**Third-party / agent context:** [`skills/AGENTS_SECURITY_CONTACT.md`](../skills/AGENTS_SECURITY_CONTACT.md).

## Audit Status

Contracts have not yet been formally audited. A third-party audit is recommended before mainnet deployment with significant TVL.

**User-facing summary:** [`security-posture.md`](./security-posture.md) (footer link from the dApp).

For an **in-repo** invariant matrix, trust assumptions, and mapping to automated tests, see [contracts-security-audit.md](./contracts-security-audit.md). **Frontend / indexer off-chain trust** (remediation [#376](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/376) / [#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/378)): [security-model.md § Off-chain trust boundaries](./security-model.md#off-chain-trust-boundaries-frontend--indexer).
