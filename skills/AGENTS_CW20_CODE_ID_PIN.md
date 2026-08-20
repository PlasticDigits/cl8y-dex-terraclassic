# Agent playbook: listed CW20 `code_id` pin (GitLab #582)

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

**#581 / 8266:** listing allowed **only if** factory **1.9.0** + pair **1.15.0** (this control) are live on that factory, **or** SpaceUSD wasm admin is cleared, **or** wrap-to-10184. Do not `AddWhitelistedCodeId 8266` while #582 is open without that statement.

Do **not** add pair balance-delta / FoT swap math (H-01).

## Invariant F6

1. **Pin at instantiate** — pair queries live `ContractInfo.code_id` for both assets and stores `ASSET_CODE_IDS` (order matches `asset_infos`). Query: `GetAssetCodeIds`.
2. **Write-path re-check** — swap / provide / withdraw / limit place+fill / cancel / claim abort unless live id **equals the pin** **and** factory `IsCodeIdWhitelisted` is true.
3. **Fail closed** — `ContractInfo` or factory query errors → `AssetCodeIdGuardUnavailable` (same posture as blacklist guard).
4. **Refresh** — factory-only `RefreshAssetCodeIds` re-pins live ids **only if both are still whitelisted**. Governance: `RefreshPairAssetCodeIds` / `RefreshPairAssetCodeIdsBatch`.
5. **Migrate** — factory **1.9.0** first (adds `IsCodeIdWhitelisted`), then pair **1.15.0** (pins + re-check). Pair migrate backfills missing pins from live `ContractInfo`.
6. **No FoT math** — migrated taxed/rebase wasm must fail closed, not “work”.

## Versions

| Contract | cw2 |
|----------|-----|
| Factory | **1.9.0** |
| Pair | **1.15.0** |

## Operator sequence (honest token upgrade)

1. Source-review the new wasm; `AddWhitelistedCodeId` the new id (keep the old id listed until Refresh finishes).
2. Token wasm admin `MsgMigrateContract` instances.
3. Governance `RefreshPairAssetCodeIds` (or Batch until `has_more=false`).
4. Optional `RemoveWhitelistedCodeId` the old template — remaining pairs still on that pin freeze until they migrate + refresh.

**Incident (discovered buggy template):** `RemoveWhitelistedCodeId` immediately. Do not Refresh onto it. Pause/blacklist remain available but are not required for the freeze.

## Do not

- Refresh pins onto an unlisted live id (the pair rejects it).
- Skip factory 1.9.0 migrate — pair write paths fail closed without `IsCodeIdWhitelisted`.
- Treat existing protocol-admin pairs as third-party issuer risk; still pin them.
- Implement (D) indexer watch as a substitute for (A).

## Verification

```bash
make verify-issue-582
```

No LocalTerra required. Tests: `asset_code_id_pin_tests::*` (honest CreatePair → FoT migrate → swap fails; whitelist removal freeze; pin vs other whitelisted template until Refresh; pair migrate backfill).

## Related

- [`AGENTS_HOOK_CW20_OPS.md`](./AGENTS_HOOK_CW20_OPS.md) — H-01 FoT prohibition (no balance-delta math)
- [`docs/runbooks/cw20-whitelist-policy.md`](../docs/runbooks/cw20-whitelist-policy.md)
- [`docs/runbooks/cw20-code-id-ops.md`](../docs/runbooks/cw20-code-id-ops.md)
- [`docs/contracts-terraclassic.md` § Asset CW20 code_id pin](../docs/contracts-terraclassic.md#asset-cw20-code-id-pin-gitlab-582)
- Invariant **F6** — [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md)
- Parent ops [#558](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/558); gate on [#581](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/581)
