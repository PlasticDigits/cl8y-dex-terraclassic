# Agent playbook: community tax catalog API (GitLab #594)

Use when changing community-token ingest, `GET /api/v1/community-tokens`, or `community_tax` on token detail.

Sibling: dApp [#593](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593); on-chain events [#592](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592); post-merge Coolify catalog [#602](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/602) ([`AGENTS_POST_MERGE_OPS_602.md`](./AGENTS_POST_MERGE_OPS_602.md)); ExemptionDirectory tax skip [#609](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/609) ([`AGENTS_COMMUNITY_TAX_EXEMPT.md`](./AGENTS_COMMUNITY_TAX_EXEMPT.md) — do not treat a catalog event as on-chain exempt); route/solve net ranking [#615](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/615) ([`AGENTS_INDEXER_TAX_AWARE_ROUTING.md`](./AGENTS_INDEXER_TAX_AWARE_ROUTING.md) — catalog `buy_bps` / `sell_bps` feed the solver; list handlers stay no request-path LCD). LocalTerra deploy writes **local** `COMMUNITY_TAX_CODE_ID` / `COMMUNITY_TOKEN_LAUNCHER` / `CMM_GOVERNANCE_ADDR=test1` / `COMMUNITY_TAX_OPTION2_CODE_IDS=<local id>` ([#620](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/620), [`AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md`](./AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md) **L620-5**) — do not pin columbus-5 **11619** against local instances. Leftover ingest after !414: [#624](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/624) ([`AGENTS_POST_MERGE_OPS_624.md`](./AGENTS_POST_MERGE_OPS_624.md) **M624-3**).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#594**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/594) | Indexer catalog |
| [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) | **I594-1–I594-10** |
| [`community_tokens.rs`](../indexer/src/api/community_tokens.rs) | HTTP |
| [`indexer/community_tokens.rs`](../indexer/src/indexer/community_tokens.rs) | Parse + 60s LCD probe |
| [`AGENTS_COMMUNITY_TAX_CW20.md`](./AGENTS_COMMUNITY_TAX_CW20.md) | Event `action` names |
| [`AGENTS_FRONTEND_CREATE_TOKEN.md`](./AGENTS_FRONTEND_CREATE_TOKEN.md) | dApp consumer |

## Invariants **I594-1–I594-10**

1. **I594-1 — configured.** Unset `COMMUNITY_TAX_CODE_ID` / `COMMUNITY_TOKEN_LAUNCHER` / `CMM_GOVERNANCE_ADDR` → list `{ configured: false, items: [] }`; detail **404**.
2. **I594-2 — default list.** `attested_cmm=true` only. Ops: `?include_unattested=1`.
3. **I594-3 — attest.** `code_id` + `admin` from LCD `ContractInfo` (not event fields). `launcher_tx` only when wasm `_contract_address` == env launcher. `GetLauncherOrigin` must match launcher. Adopted tokens ([#626](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/626)) may attest with `GetMigrateOrigin` (allowlisted source cw2) **instead of** `launcher_tx` — never fake `launcher_tx`. See [`AGENTS_FRONTEND_TOKEN_MIGRATE.md`](./AGENTS_FRONTEND_TOKEN_MIGRATE.md) **M626-6**. Catalog is **single-id** — Coolify/indexer bake the current listed tax pin (**11630** after #635; **11626** was the #628 store). Post-merge leftover live: [#628](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/628) ([`AGENTS_POST_MERGE_OPS_628.md`](./AGENTS_POST_MERGE_OPS_628.md)).
4. **I594-4 — no scrape.** Ingest launcher `create_token_ready` / `enable_feature`, token `migrate-adopt` (no `launcher_tx`), and catalogued token `update_settings` / `mint`. Do not index every CW20.
5. **I594-5 — settings vs SKU.** Event `kind=settings_fee` is separate from `sku_unlock`.
6. **I594-6 — pagination.** Same clamp as tokens: `limit` 1–100 (default 50), `offset` > 10_000 → **400**.
7. **I594-7 — no request-path LCD.** List/detail/events read Postgres only. Probe loop is 60s background.
8. **I594-8 — no new public POST.** Read-only HTTP (+ existing route/solve POST).
9. **I594-9 — token detail.** `GET /tokens/{addr}` may include `community_tax` when catalogued; otherwise omit/null.
10. **I594-10 — not F6.** Catalog is not a substitute for factory whitelist / code-id pin.

## Verify

```bash
make setup-indexer-postgres
make verify-issue-594
make verify-issue-602
make verify-issue-620
make verify-issue-624
```

## Do not

- Trust event `code_id`.
- Mark `launcher_tx` from a non-launcher emitter.
- Treat 11612/11613 instantiates as catalog tokens.
- Treat a catalog / wasm event as on-chain `MANAGER_EXEMPT` without the token already in catalog ([#609](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/609) **E609** / **I594-3**).
- Bind-mount `indexer/` into root Docker to run cargo.
