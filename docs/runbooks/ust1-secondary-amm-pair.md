# Runbook: UST1 secondary AMM pair (GitLab #508)

Create and seed a **secondary** CL8Y factory pair for UST1 price discovery — preferred **UST1/vFDUSD**, optional **UST1/cUSTC** — **or** record an explicit **product waiver**. Parent track: [#502](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/502) Phase 4.

Oracle mint/redeem remains primary via ust1-window (`/ust1`). The AMM must **never** be presented as the mint/redeem path.

**Agent playbook:** [`skills/AGENTS_UST1_SECONDARY_AMM.md`](../../skills/AGENTS_UST1_SECONDARY_AMM.md)

## Invariants

| ID | Rule |
|----|------|
| **U1** | AMM is **secondary**. `/ust1` (ust1-window CW20 `Send` + `effective_swap`) remains primary mint/redeem. UI/docs must not market AMM as “mint UST1”. |
| **U2** | Prefer CW20 code **10184** tokens already on the factory whitelist (**SL1**). Do not add code IDs without [`cw20-whitelist-policy.md`](./cw20-whitelist-policy.md). |
| **U3** | Pair assets are CW20 only — **UST1/vFDUSD** and/or **UST1/cUSTC**. Native `uusd` / `uluna` are rejected by the factory. |
| **U4** | Seed liquidity is **smoke/discovery** sized; document amounts. Do not imply deep peg defense vs the oracle window. |
| **U5** | Indexer accepts **factory-provenance** pairs only (invariant **P1**). Foreign emitters are rejected. |
| **U6** | Do **not** fold UST1 into soft-launch gemstone defaults ([`mainnet-soft-launch-defaults.sh`](../../scripts/lib/mainnet-soft-launch-defaults.sh) / **SL5**). Use this runbook’s scripts + `deployments/ust1-secondary-pair/`. |
| **U7** | Close #508 via **Path A** (live seeded pair + txs) **or Path B** (explicit product waiver on #508 and #502). No silent deferral. |

## Known mainnet anchors

| Role | Address |
|------|---------|
| Factory | `terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea` |
| Router | `terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw` |
| Fee-discount | `terra1wcczsdk7jwj99n3my6wx8wr4ee0hn6yaapgd792lgx5elrdtrn2scfnecz` |
| UST1 | `terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72` |
| vFDUSD | `terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3` |
| cUSTC | `terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch` |

Defaults: [`scripts/lib/ust1-secondary-pair-defaults.sh`](../../scripts/lib/ust1-secondary-pair-defaults.sh).

## Path A — create + seed (preferred)

### Preconditions

1. Both assets `token_info` OK; code ID **10184** (U2).
2. Creator has **≥ `pair_creation_fee_uluna`** (default 100 LUNC) plus gas.
3. Creator holds both CW20 balances ≥ seed amounts (default `1000000` raw = **1.0** per side — U4).
4. UST1 inventory typically comes from `/ust1` window deposit (vFDUSD → UST1). If `total_supply(UST1)=0`, Path A is blocked until window mint inventory exists — use Path B interim waiver rather than an empty pool.

### Commands

```bash
# Preflight + plan (no txs)
DRY_RUN=1 ./scripts/add-ust1-secondary-pair.sh

# Live create + seed (needs TERRAD_HOST_KEYRING_PASS or TTY unlock)
UST1_SEC_PAIR_LEG=vfdusd ./scripts/add-ust1-secondary-pair.sh
# optional: UST1_SEC_PAIR_LEG=custc
# optional larger smoke seed: UST1_SEC_SEED_AMOUNT_A=… UST1_SEC_SEED_AMOUNT_B=…
# create without seed is blocked unless: UST1_SEC_SKIP_LP=1 UST1_SEC_ALLOW_UNSEEDED=1
# post–governance handoff: set_discount_registry may need multisig, or UST1_SEC_ALLOW_DISCOUNT_FAIL=1
```

Script writes:

- `deployments/ust1-secondary-pair/addresses.env` — pair address, create/seed txs, seed amounts
- Appends a section to `deployments/ust1-secondary-pair/deploy-trace.md`

Optional: UI `/create` (`CreatePairPage`) can create the pair; still record seed txs and amounts here.

### Post-create checks

1. Factory `Pair { asset_infos }` returns the pair contract.
2. Pair `Pool {}` → `total_share > 0`.
3. Indexer lists the pair (auto-discovery / `sync_all_pairs`, **P1** / **U5**).
4. Trade/Swap on `dex.cl8y.com` can quote + small round-trip; record smoke tx on #508.
5. `/ust1` copy (when shipped, [#506](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/506)) may link Trade/Swap as **secondary market** only — never as mint/redeem (**U1**).

### LocalTerra rehearsal

```bash
make deploy-local   # or make setup-cloud-localterra
./scripts/seed-ust1-secondary-pair-local.sh
# writes deployments/ust1-secondary-pair/local-addresses.env
make verify-issue-508
```

Local tokens are **stand-ins** (same symbols) for flow verification — not columbus-5 addresses.

## Path B — product waiver

When inventory or product timing blocks Path A:

1. Write rationale + deferred scope + **revisit trigger** on #508 **and** #502.
2. Keep [`deployments/ust1-secondary-pair/PRODUCT_WAIVER.md`](../../deployments/ust1-secondary-pair/PRODUCT_WAIVER.md) in sync.
3. Do **not** create an unseeded/dust market that looks like a live peg venue (**U4** / issue guardrail).

Current interim waiver (inventory): see that file — revisit when `total_supply(UST1) > 0` and an operator wallet holds both legs for seed.

## Soft-launch boundary

Soft-launch SL1–SL7 and the gemstone catalog stay unchanged. Post–SL economic pairs use this runbook. See [`mainnet-soft-launch.md`](./mainnet-soft-launch.md) § “Adding economic tokens later” and [`deployment-guide.md`](../deployment-guide.md) §6.

## Verification

```bash
make verify-issue-508
# Optional live LocalTerra fixture (needs chain + .env.local):
VERIFY508_LOCAL=1 make verify-issue-508
# Optional mainnet read-only presence check:
VERIFY508_MAINNET=1 make verify-issue-508
```

## Related

- Issue [#508](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/508) · parent [#502](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/502)
- `/ust1` UI [#506](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/506) · wrap env [#507](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/507)
- Indexer provenance: [`docs/indexer-invariants.md`](../indexer-invariants.md) **P1**, [`skills/AGENTS_FACTORY_ADDRESS_GUARD.md`](../../skills/AGENTS_FACTORY_ADDRESS_GUARD.md)
- Frontend secondary-market copy helpers: `frontend-dapp/src/utils/ust1SecondaryMarket.ts`
- CG/CMC listing note if public markets: [`docs/CG_CMC_COMPLIANCE.md`](../CG_CMC_COMPLIANCE.md)
