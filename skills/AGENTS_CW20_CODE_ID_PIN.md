# Agent playbook: listed CW20 `code_id` pin (GitLab #582 / #584)

Use when migrating factory/pair wasm, adding a CW20 code id to the factory whitelist, listing a third-party token (including **#581 / 8266 SpaceUSD**), or debugging swaps that fail with `Asset CW20 code_id drifted` / `not factory-whitelisted`.

## Decision (issue notes)

| Option | Status |
|--------|--------|
| **(B) Pin exact `code_id` at CreatePair** | **Chosen** — token behavior stays the listing-time template |
| **(A) Re-check factory whitelist on write paths** | **Chosen** — `RemoveWhitelistedCodeId` freezes pairs still on that template |
| **(C) Refuse wasm admin** | Not implemented (listing policy only; does not close 6036 CreatePair) |
| **(D) Indexer / ops watch** | **Rejected** — (A) covers the same freeze with no detection window |
| **(E) Do not whitelist 8266** | **Rejected** as the 8266 policy; quality tokens may need admin/upgrade |

**Severity:** **High** for permissionless 6036+migrate. Residual risk on protocol-admin 10184/6036 is our-key upgrade risk (still fail-closed until Refresh).

**#581 / 8266:** listing allowed **only after** factory **1.9.0** + pair **1.15.0** (this control) are **migrated live** on that factory, **or** SpaceUSD wasm admin is cleared, **or** wrap-to-10184. Merging the contract MR / this playbook is not enough — operators must run [`scripts/upgrade-582-code-id-pin.sh`](../scripts/upgrade-582-code-id-pin.sh) on columbus-5 ([#584](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/584)). Launch checklist **BLOCK** remains until that run’s smoke table is on [#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391).

Do **not** add pair balance-delta / FoT swap math (H-01).

Factory/pair **F6** (this pin) is not faucet **F6** (deploy key remains primary CW20 minter).

## Invariant F6

1. **Pin at instantiate** — pair queries live `ContractInfo.code_id` for both assets and stores `ASSET_CODE_IDS` (order matches `asset_infos`). Query: `GetAssetCodeIds`.
2. **Write-path re-check** — swap / provide / withdraw / limit place+fill / cancel / claim abort unless live id **equals the pin** **and** factory `IsCodeIdWhitelisted` is true.
3. **Fail closed** — `ContractInfo` or factory query errors → `AssetCodeIdGuardUnavailable` (same posture as blacklist guard).
4. **Refresh** — factory-only `RefreshAssetCodeIds` re-pins live ids **only if both are still whitelisted**. Governance: `RefreshPairAssetCodeIds` / `RefreshPairAssetCodeIdsBatch`.
5. **Migrate** — factory **1.9.0** first (adds `IsCodeIdWhitelisted`), then pair **1.15.0** (pins + re-check). Pair migrate backfills missing pins from live `ContractInfo`. **Enforced by** [`scripts/upgrade-582-code-id-pin.sh`](../scripts/upgrade-582-code-id-pin.sh): the script aborts before any pair 1.15.0 migrate unless factory cw2 ≥ 1.9.0 **and** `IsCodeIdWhitelisted` succeeds. Do **not** copy the #514 pairs-first order.
6. **No FoT math** — migrated taxed/rebase wasm must fail closed, not “work”.
7. **Exit-path policy (keep)** — cancel / claim / withdraw stay gated. Documented tradeoff + unfreeze (pause-through-refresh) live in [`docs/runbooks/cw20-code-id-ops.md`](../docs/runbooks/cw20-code-id-ops.md). Opening exits is a **follow-up contract issue**, not an ops-script change.

## Versions

| Contract | cw2 |
|----------|-----|
| Factory | **1.9.0** |
| Pair | **1.15.0** |

## Operator sequence (factory/pair wasm upgrade)

```bash
# Read-only columbus-5 ContractInfo probe (paste onto #584 / #391):
./scripts/qa/probe-columbus5-contract-info.sh

# LocalTerra after make deploy-local:
UPGRADE582_LOCAL=1 ./scripts/upgrade-582-code-id-pin.sh

# columbus-5 (governance / wasm-admin key):
./scripts/upgrade-582-code-id-pin.sh
```

The script: probes `GET /cosmwasm/wasm/v1/contract/{addr}` for factory + every listed asset → stores wasm → migrates **factory 1.9.0** → asserts cw2 + `IsCodeIdWhitelisted` → paginates `pairs` at `limit: 30` with `start_after` = last `asset_infos` → reconciles `GetPairCount` → smoke `GetAssetCodeIds` + `HybridSimulation` (queries are **ungated**; a quote is not “pair is tradable”).

## Operator sequence (honest token upgrade)

1. Source-review the new wasm; `AddWhitelistedCodeId` the new id (keep the old id listed until Refresh finishes).
2. Token wasm admin `MsgMigrateContract` instances.
3. **`SetPairPaused { paused: true }` first** if the token already drifted (keep paused through refresh). See incident runbook.
4. Governance `RefreshPairAssetCodeIds` (or Batch until `has_more=false`). Batch skip-via-`start_after` is in the runbook — do not Refresh onto an unlisted / FoT live id.
5. Private rebalance while still paused, then `SetPairPaused { paused: false }`. Do not announce the unpause height in public chat.
6. Optional `RemoveWhitelistedCodeId` the old template — remaining pairs still on that pin freeze until they migrate + refresh. **Never lead with `RemoveWhitelistedCodeId(10184)`** (protocol-wide halt).

**Incident (discovered buggy template):** do **not** Refresh onto it. Pause (`SetPairPaused`) / `BlacklistPair` / `BlacklistToken`. See [`cw20-code-id-ops.md`](../docs/runbooks/cw20-code-id-ops.md).

## Do not

- Refresh pins onto an unlisted live id (the pair rejects it).
- Skip factory 1.9.0 migrate — pair write paths fail closed without `IsCodeIdWhitelisted`.
- Treat existing protocol-admin pairs as third-party issuer risk; still pin them.
- Implement (D) indexer watch as a substitute for (A).
- Treat `make verify-issue-582` pin tests as “columbus-5 migrate ran”.
- De-whitelist 10184 as the default incident response.

## Verification

```bash
make verify-issue-584   # script bash -n, DRY_RUN factory-assert, pagination mock, runbook greps
make verify-issue-582   # pin tests + #584 ops (fails if upgrade script is deleted)
```

No LocalTerra required for those targets. Live rehearsal: `UPGRADE582_LOCAL=1` after `make deploy-local`. Columbus-5 read-only probe: `./scripts/qa/probe-columbus5-contract-info.sh`.

Tests: `asset_code_id_pin_tests::*` (honest CreatePair → FoT migrate → swap fails; whitelist removal freeze; pin vs other whitelisted template until Refresh; pair migrate backfill).

## Related

- [`AGENTS_HOOK_CW20_OPS.md`](./AGENTS_HOOK_CW20_OPS.md) — H-01 FoT prohibition (no balance-delta math)
- [`docs/runbooks/cw20-whitelist-policy.md`](../docs/runbooks/cw20-whitelist-policy.md)
- [`docs/runbooks/cw20-code-id-ops.md`](../docs/runbooks/cw20-code-id-ops.md) — exit-path **keep**, unfreeze, batch skip
- [`docs/runbooks/emergency-commands.md`](../docs/runbooks/emergency-commands.md) — `SetPairPaused` through refresh
- [`docs/runbooks/wasm-admin-migration.md`](../docs/runbooks/wasm-admin-migration.md)
- [`docs/runbooks/launch-checklist.md`](../docs/runbooks/launch-checklist.md) — **BLOCK** until columbus-5 migrate has **run**
- [`docs/contracts-terraclassic.md` § Asset CW20 code_id pin](../docs/contracts-terraclassic.md#asset-cw20-code-id-pin-gitlab-582)
- Invariant **F6** — [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md)
- Parent ops [#558](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/558); gate on [#581](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/581); rollout [#584](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/584)
