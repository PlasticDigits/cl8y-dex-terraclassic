# Agent playbook: UST1 + wrap production ops (GitLab #503)

Use when hardening **production monitoring**, **pause playbooks**, **address registries**, or **go/no-go checks** for the UST1 oracle window and cLUNC/cUSTC wrap stack after Phases 2–4 on `dex.cl8y.com`.

**Parent:** [GitLab **#502**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/502)  
**This issue:** [GitLab **#503**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/503)  
**Children (Phase 4):** [#506](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/506) `/ust1`, [#507](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/507) wrap UX, [#508](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/508) secondary AMM.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [`docs/runbooks/ust1-wrap-production-ops.md`](../docs/runbooks/ust1-wrap-production-ops.md) | Ops hub — invariants **O1–O8**, checks, oracle env, on-call roles |
| [`docs/runbooks/wrap-mapper-pause.md`](../docs/runbooks/wrap-mapper-pause.md) | Columbus-5 wrap-mapper pause/unpause + smoke record |
| [`deployments/mainnet-ust1-wrap/REGISTRY.md`](../deployments/mainnet-ust1-wrap/REGISTRY.md) | Canonical Phase 2–4 addresses (no secrets) |
| [`deployments/mainnet-ust1-wrap/coolify.env.example`](../deployments/mainnet-ust1-wrap/coolify.env.example) | Coolify `VITE_*` pack |
| [`scripts/check-ust1-wrap-ops-health.sh`](../scripts/check-ust1-wrap-ops-health.sh) | Read-only LCD health probe |
| [`docs/user-incident-faq.md`](../docs/user-incident-faq.md) | UST1 + wrap pause user copy |
| Upstream | [ust1-window](https://gitlab.com/PlasticDigits/ust1-window) `verify_oracle_operator_env.sh` / `oracle-service`; [ustr-cmm](https://gitlab.com/PlasticDigits2/ustr-cmm) treasury governance |

## Invariants (O1–O8)

| ID | Rule |
|----|------|
| **O1** | Addresses only from `REGISTRY.md`; Coolify must match; no LocalTerra defaults on prod. |
| **O2** | Never disable oracle age checks to “fix” staleness — restore operator/RPCs. |
| **O3** | Pause is primary: window `set_paused`, oracle pause, wrap-mapper `set_paused`, treasury `set_wrapping_paused`. |
| **O4** | Monitor treasury vFDUSD **balance and allowance** to ust1-window. |
| **O5** | Wrap solvency: treasury native ≥ CW20 supply; pause unwrap on breach. |
| **O6** | Confirm on-chain `governance` before pause. Mapper + CMM treasury **app** gov is DEX 2-of-3 (`terra1zlmv2…`) after #525/#526; window/oracle stay `cl8y2_admin`. Wasm admin on mapper/treasury is still the EOA. |
| **O7** | Do not widen factory CW20 whitelist in ops panic. |
| **O8** | On-call roles named; silence-alert + pause-drill evidence on #503. |

Do not confuse with UI **U1–U8** ([`AGENTS_UST1_WINDOW_UI.md`](./AGENTS_UST1_WINDOW_UI.md)) or Coolify/UX **W1–W7** ([`AGENTS_MAINNET_WRAP_ENABLEMENT.md`](./AGENTS_MAINNET_WRAP_ENABLEMENT.md)).

## Rules of thumb

1. **Registry first** — edit `deployments/mainnet-ust1-wrap/REGISTRY.md` when addresses change; mirror Coolify + upstream repos.
2. **Read-only probes** — `./scripts/check-ust1-wrap-ops-health.sh` before any governance pause.
3. **Oracle bot** — `verify_oracle_operator_env` runs in **ust1-window**; Coolify `ORACLE_MAX_SILENCE_SECS=21600` (6h, ≤ `max_oracle_age_sec`); attach silence-alert evidence on #503.
4. **Wrap pause smoke** — LocalTerra `make smoke-wrap-mapper-pause`; mainnet in `wrap-mapper-pause.md`.
5. **429 ≠ wrap rate limit** — indexer HTTP 429 is off-chain; read wrap-mapper `rate_limit` / `set_rate_limit` in the ops runbook.
6. **No secrets in git** — Coolify/host secrets only; on-call people names stay private.
7. **Agents must not broadcast governance txs** — prepare commands from the runbook; humans with wrap-stack / DEX keys sign.

## Quick commands

```bash
make verify-issue-503
VERIFY503_MAINNET=1 make verify-issue-503
./scripts/check-ust1-wrap-ops-health.sh
UST1_OPS_STRICT_PAUSE=1 UST1_OPS_STRICT_STALE=1 ./scripts/check-ust1-wrap-ops-health.sh
UST1_OPS_STRICT_PAUSE=1 UST1_OPS_STRICT_STALE=1 UST1_OPS_STRICT_INVENTORY=1 ./scripts/check-ust1-wrap-ops-health.sh
make smoke-wrap-mapper-pause   # LocalTerra; needs deploy-local
```

## Related

- [`AGENTS_UST1_WINDOW_UI.md`](./AGENTS_UST1_WINDOW_UI.md) (#506)
- [`AGENTS_MAINNET_WRAP_ENABLEMENT.md`](./AGENTS_MAINNET_WRAP_ENABLEMENT.md) (#507)
- [`AGENTS_WRAP_MAPPER_SPLIT_FEES.md`](./AGENTS_WRAP_MAPPER_SPLIT_FEES.md) (#516 — `fee_wrap_bps` / `fee_unwrap_bps` + retune)
- [`AGENTS_UST1_SECONDARY_AMM.md`](./AGENTS_UST1_SECONDARY_AMM.md) (#508)
- [`AGENTS_MAINNET_SOFT_LAUNCH.md`](./AGENTS_MAINNET_SOFT_LAUNCH.md)
- [`AGENTS_KEY_CUSTODY.md`](./AGENTS_KEY_CUSTODY.md)
- [`AGENTS_FRONTEND_SWAP_SAFETY_CTA.md`](./AGENTS_FRONTEND_SWAP_SAFETY_CTA.md)
- [`AGENTS_USER_INCIDENT_FAQ.md`](./AGENTS_USER_INCIDENT_FAQ.md)
