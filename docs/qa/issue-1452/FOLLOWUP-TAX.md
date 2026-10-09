# Local community-tax quotes: follow-up scope

Related: [#1452](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1452),
[LQ1452](../../../skills/AGENTS_LOCAL_SWAP_QUOTES.md),
[RT1381](../../../skills/AGENTS_ROUTE_LOCAL_TAX.md),
[LG1390](../../../skills/AGENTS_ROUTE_LAUNCH_GUARDS.md),
[agent checklist](README.md), and [solver contract](../../route-solver.md#local-quote-contract-1452).

`indexer/src/api/local_quote/tax.rs` stores independently scheduled observations
for immutable registered implementations with funding version 1 and the configured
router/factory. It rechecks code, capability, config and features after dependent
reads. Pair registration, bounded tax sinks, actor protocol/manager flags and
balances are observed outside requests. Failed required observations omit support.
The policy identity binds validated token configuration and features; the router's
changing route-id counter does not invalidate otherwise compatible pair observations.
The request engine performs checked integer arithmetic on those observations.

## Economic invariants

- Initial offer is the entire authorized gross debit. At each sell, economic input
  is the greatest integer `S` such that `S + floor(S * sell_bps / 10000) <= gross`.
  Pair credit, swap hook input and economic pool/book budgets agree exactly.
- Each buy deducts `floor(post_hook_output * buy_bps / 10000)` once. That net output
  alone funds the next sell and is the final output/minimum basis. No intermediate
  wallet balance, allowance or router dust funds taxes.
- `tax_kind=route_local_v1`, funding version and per-hop gross/economic/tax/residue
  metadata and exact `declared_hybrid` budgets/fill cap/hint are pinned with the observation generation. Frontend validation checks
  maximal sell sizing, accounting identities, net floors and economic hybrid budgets.
  CW20/native signing serializes `execute_route_swap_operations` with
  `funding_version: 1` from pinned metadata, without fresh capability discovery.
  A different final wallet recipient requires its own observation context and is
  unsupported by the current local signing interface.
- Manual gross ratios rescale exactly as the router: economic book input is
  `floor(economic * declared_book / gross)`; pool takes the remainder. Frontend
  checks that precise transformation before retaining the signed operations.
- Pool receive sizing reverses buy tax, deducting hooks, commission, AMM sizing and
  sell tax, then checks the exact selected input through local forward scoring.
  Arithmetic uses wide integers and bounded searches, including Uint128 limits.
- Manager/directory exemption is per token and only waives tax. Protocol addresses
  cannot be routed traders. Capable-token repetition is rejected even at zero rates
  or with manager exemption. Fresh known balances enforce configured wallet caps;
  unknown actor identity/balance cannot authorize a wallet-specific quote.
  Intermediate net stays in the router; cap checks apply to its purchased rounding
  residue and final delivery. Original-input residue restores authenticated owned
  funds and keeps the protocol's distinct cap exception.
- Native entry/exit preserves tax metadata and applies wrapper fees to already net
  route output. Fair references continue to use authorized gross wallet input.

Local signers serialize `execute_route_swap_operations` with `funding_version:1`
from pinned metadata for both CW20 and native plans. They forbid a replacement
recipient whose wallet-cap context was not observed.

## Explicit unsupported observations

Stateful cooldown is unsupported because the current public token queries do not
expose the last-trade state needed to reproduce it. Enabled nonzero cooldown fails
closed. Disabled trading also fails closed, including manager-exempt traders.

Enabled AutoLP is unsupported: a tax-triggered AutoLP action can change reserves
before later swap work, and this snapshot does not predict its threshold/settlement.
Legacy funding versions, unknown code/router/factory identity, unsupported sinks,
missing named-actor flags and missing required cap balances remain unavailable.
A new wallet becomes eligible only through independently scheduled observation;
requests never enqueue an actor discovery or accelerate ingestion. These exclusions
remain engineering follow-ups and prevent complete issue/deployed acceptance.

## Agent verification checklist

Use a dedicated worktree and the disposable DB rules in the main [checklist](README.md).
Run available checks before recording a skip; repeat after corrections.

- [ ] Run `cargo test --manifest-path indexer/Cargo.toml --lib local_quote::tax`.
  Inspect maximal funded integer sizing/overflow, exact two-hop intermediate net,
  manager-vs-protocol identity, cap rejection, capability repetition, config-merge
  invalidation and inverse minimality/forward sufficiency assertions.
- [ ] Run `bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/vitest run src/utils/localRouteQuote.test.ts`.
  Inspect authorized gross signing, provider-call count zero, economic hybrid
  budgets and malformed tax accounting rejections.
- [ ] Run `make verify-issue-1452`, `make verify-issue-1381` and
  `make verify-issue-1390`; record command, revision, UTC time, counts and results.
- [ ] With synthetic independently observed v1 tokens, compare direct/multihop,
  taxed-intermediate, pool/hybrid, native and receive-sizing local arithmetic to
  LocalTerra execution. Test protocol/manager actors and cap boundaries; preserve
  original-input refund ownership and atomic rollback. Never broadcast production QA.
- [ ] Poison missing/stale/future actor/config/pair data and provider outages;
  verify unavailable execution, no stale cache approval and zero request RPC.
- [ ] Measure normal-transport useful quote latency and production revision
  independently. Source correctness and skipped deployment do not prove live acceptance.

Overall review may be **PASS WITH SKIPS** when all executed checks pass. For each
unavailable check, record the attempted command, concrete missing capability and
unverified criterion. Leave that box unchecked. An executed accounting/identity
failure stays **FAIL** and must be corrected/retested, never converted to a skip.
