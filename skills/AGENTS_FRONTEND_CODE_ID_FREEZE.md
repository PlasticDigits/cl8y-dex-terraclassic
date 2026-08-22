# Agent playbook: F6 code-id freeze on dApp + indexer (GitLab #585)

Use when changing Swap / Trade / Pool / Charts freeze banners, `route/solve` hop filtering, pair `code_id_frozen`, or humanizing `AssetCodeIdDrift` / whitelist / guard errors.

On-chain F6 (pin + write-path re-check) is **[#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582)** / **[#584](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/584)**. This issue is **visibility**: indexer must not treat frozen hops as executable; the dApp must not show a generic failed tx. **Do not** treat this indexer watch as a substitute for on-chain **A** (write-path re-check). See [`AGENTS_CW20_CODE_ID_PIN.md`](./AGENTS_CW20_CODE_ID_PIN.md) — option **(D)** remains rejected for security.

Issue **#585 is implemented**. On-chain F6 pin stays live regardless of 8266 listing. This UX MR does not replace write-path fail-closed.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#585**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/585) | Surface F6 freeze on dApp + `route/solve` |
| [`docs/frontend.md` § Code-id freeze](../docs/frontend.md#code-id-freeze-gitlab-585) | Invariants **F585-1–F585-8** |
| [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) | `code_id_frozen` + hop exclude |
| [`asset_code_id_freeze.rs`](../indexer/src/indexer/asset_code_id_freeze.rs) | Process-local freeze cache + 60s LCD probe |
| [`assetCodeIdFreeze.ts`](../frontend-dapp/src/utils/assetCodeIdFreeze.ts) | Copy + `evaluateLivePins` |
| [`AGENTS_CW20_CODE_ID_PIN.md`](./AGENTS_CW20_CODE_ID_PIN.md) | On-chain **F6**; ops script; no FoT math |

## Invariants **F585-1–F585-8**

1. **F585-1 — route exclude.** `GET/POST /api/v1/route/solve` skips pairs whose live `ContractInfo.code_id` ≠ pin **or** live id is not factory-whitelisted. Filter at **path enumeration** (`find_path` + `build_adjacency`), not only graph snapshot load (15s TTL would lag inject tests).
2. **F585-2 — catalog flag.** Pair list/detail (and token-pairs) expose `code_id_frozen: bool`. Frozen pairs **stay listed** so `/charts` / `/pool` can show state.
3. **F585-3 — fail-open LCD for routing/UI.** Probe errors keep last known membership (unknown ≠ frozen). Pre-1.15.0 `GetAssetCodeIds` (`unknown variant` / unpinned) → **not frozen**. On-chain execute still **fail-closes**.
4. **F585-4 — no LCD on request path.** Indexer freeze set is a process-local `HashSet`; background probe every **60s**. Do not query LCD inside `route/solve`.
5. **F585-5 — humanize execute errors.** Swap / Trade / Pool map `Asset CW20 code_id drifted`, `not factory-whitelisted`, `Asset code_id guard unavailable`, and unpinned pins to retail copy — not a generic failed tx.
6. **F585-6 — surfaces + quotes-still-appear.** `/`, `/trade`, `/pool`, `/charts`, `/limits` show freeze state when known (indexer field **or** LCD). Banner copy: quotes can still appear; swaps / LP / limits blocked. CTA **Market frozen**.
7. **F585-7 — no un-gate exits.** This issue does **not** open on-chain cancel / claim / withdraw. Exit-path **keep** stays in [`cw20-code-id-ops.md`](../docs/runbooks/cw20-code-id-ops.md).
8. **F585-8 — no FoT math.** Do **not** add pair balance-delta / fee-on-transfer swap math (**H-01**). Queries (`Simulation` / `HybridSimulation`) stay ungated by design.

## Rules of thumb

- A successful LCD quote is **not** “pair is tradable”.
- Indexer hint `code_id_frozen` **ORs** with the LCD probe (`usePairCodeIdFreeze`).
- Direct pair LCD sim on Swap can still quote a hop the user already selected; `route/solve` will not **propose** frozen hops.
- Cancel/claim UI may reuse pause disable (`isPairPaused={isPaused || isPairCodeIdFrozen}`) when the freeze banner is visible — dedicated copy is optional.

## Verify

```bash
make verify-issue-585
```

Needs Postgres + `indexer/.env` for the route/pair integration tests (`make setup-indexer-postgres`). No LocalTerra. No wasm deploy.

## Related

- On-chain pin: [`AGENTS_CW20_CODE_ID_PIN.md`](./AGENTS_CW20_CODE_ID_PIN.md) (**F6**, #582 / #584)
- Hybrid quoting: [`AGENTS_HYBRID_QUOTING.md`](./AGENTS_HYBRID_QUOTING.md) — retail GET still uses `route/solve`
- Pair pause CTAs: [`docs/frontend.md` § Pair pause](../docs/frontend.md#pair-pause-disabled-ctas-sec-b05)
