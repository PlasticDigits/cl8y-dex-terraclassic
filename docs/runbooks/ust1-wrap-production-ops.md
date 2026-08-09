# UST1 + wrap production ops (Phase 5)

Operator + agent runbook for **monitoring, pause playbooks, and registry completeness** after Phases 2–4 ship on `dex.cl8y.com` ([GitLab **#503**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/503), parent [#502](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/502)).

| Phase | Track |
|-------|-------|
| 2 | [ust1-window#19](https://gitlab.com/PlasticDigits/ust1-window/-/issues/19) — window + oracle |
| 3 | [ustr-cmm#5](https://gitlab.com/PlasticDigits2/ustr-cmm/-/work_items/5) — treasury + wrap-mapper |
| 4 | [#502](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/502) — `/ust1` ([#506](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/506)), wraps ([#507](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/507)), secondary AMM ([#508](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/508)) |
| 5 | **#503** — this runbook |

**Agent playbook:** [`skills/AGENTS_UST1_WRAP_PRODUCTION_OPS.md`](../../skills/AGENTS_UST1_WRAP_PRODUCTION_OPS.md)  
**Registry:** [`deployments/mainnet-ust1-wrap/REGISTRY.md`](../../deployments/mainnet-ust1-wrap/REGISTRY.md)  
**Wrap-mapper pause detail:** [`wrap-mapper-pause.md`](./wrap-mapper-pause.md)

---

## Invariants (O1–O8)

| ID | Rule |
|----|------|
| **O1** | Canonical addresses live in [`REGISTRY.md`](../../deployments/mainnet-ust1-wrap/REGISTRY.md) (no secrets). Coolify `VITE_*` must match; never LocalTerra defaults on prod. |
| **O2** | Do **not** “fix” oracle staleness by disabling age checks — restore oracle operator / RPCs. Pause the window only as a temporary user-facing fail-closed. |
| **O3** | Primary incident controls: **ust1-window `set_paused`**, **ust1-oracle pause** (governance), **wrap-mapper `set_paused`**, treasury **`set_wrapping_paused`**. Prefer pause over panic whitelist changes (**O7**). |
| **O4** | Treasury **vFDUSD balance + allowance to ust1-window** bound withdraw capacity — monitor both. |
| **O5** | Wrap solvency (**W1**): treasury native `uluna`/`uusd` ≥ cLUNC/cUSTC `total_supply`. On breach, pause unwrap before refill. |
| **O6** | CMM wrap treasury / wrap-stack governance (`terra1xsecn4…`) **≠** DEX governance multisig `terra1zlmv2…` (**W2**). Confirm signers before pause txs. |
| **O7** | Do not silently widen factory CW20 whitelist during UST1/wrap incidents — follow [`cw20-whitelist-policy.md`](./cw20-whitelist-policy.md). |
| **O8** | On-call **roles** named (oracle bot operator + treasury/wrap governance); silence-alert and pause-drill evidence attached on [#503](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/503) (not committed secrets). |

Related UI invariants: **U1–U8** ([`ust1-window-ui.md`](./ust1-window-ui.md)), **W1–W7** ([`AGENTS_MAINNET_WRAP_ENABLEMENT.md`](../../skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md)).

---

## On-call ownership (roles)

Named by **role** (identities private per [`key-custody.md`](./key-custody.md)). Owning org: **PlasticDigits** maintainers. Fill person names privately on [#503](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/503).

| Role | Owns | Escalation |
|------|------|------------|
| **Oracle bot operator** | ust1-oracle updater key, `oracle-service` host/secrets, silence alert (`ORACLE_MAX_SILENCE_SECS` ~8h) | Key compromise → pause window → rotate operator → revoke old key |
| **Treasury / wrap governance** | CMM treasury + wrap-mapper governance (`terra1xsecn4…`), vFDUSD inventory/allowance, `set_paused` / `set_wrapping_paused` | Dual-control; never accept unsolicited migrate |
| **DEX governance signer** | Factory/router pair pause/blacklist — separate from wrap stack (**O6**) | [`emergency-commands.md`](./emergency-commands.md) |

---

## Read-only health checks

```bash
./scripts/check-ust1-wrap-ops-health.sh
UST1_OPS_STRICT_PAUSE=1 UST1_OPS_STRICT_STALE=1 ./scripts/check-ust1-wrap-ops-health.sh
make verify-issue-503
VERIFY503_MAINNET=1 make verify-issue-503
```

| Check | Signal | Healthy | Escalate |
|-------|--------|---------|----------|
| Oracle age | window `effective_swap` → `oracle.last_update_sec` vs `max_oracle_age_sec` | age ≤ max | Restore operator/RPCs (**O2**); optional window `set_paused` |
| Window pause | `effective_swap.paused` / `config.paused` | `false` | Unpause after root cause |
| Oracle pause | `effective_swap.oracle.paused` | `false` | Governance unpause oracle |
| Wrap-mapper pause | wrap-mapper `config.paused` | `false` | [`wrap-mapper-pause.md`](./wrap-mapper-pause.md) |
| Treasury wrap pause | treasury wrapping paused (`set_wrapping_paused`) | wrapping enabled | Unpause via treasury governance |
| vFDUSD inventory | CW20 balance of treasury + allowance owner=treasury spender=window | Above warn thresholds | Refill; temp window pause if drained |
| Wrap solvency | bank `uluna`/`uusd` on treasury vs cLUNC/cUSTC `total_supply` | native ≥ supply | Pause wrap-mapper (**O5**) |
| Wrap fee | wrap-mapper `config.fee_bps` | On-chain authoritative (UI must query) | Note drift vs product target; never hardcode in UI |

**Observed 2026-08-09 LCD sample:** wrap-mapper `fee_bps=200`; treasury vFDUSD balance/allowance to window were **low / zero** (health script WARNs) — refill before marketing UST1 withdraw capacity.

Indexer HTTP **429** ≠ wrap rate-limit — [user incident FAQ](../user-incident-faq.md#rate-limits).

---

## Oracle service + `verify_oracle_operator_env`

Updater bot and `scripts/verify_oracle_operator_env.sh` live in the **ust1-window** repo (`oracle-service/`).

1. On the oracle host (secrets injected, never committed), run:

   ```bash
   # Upstream path (ust1-window repo):
   ./scripts/verify_oracle_operator_env.sh
   ```

2. Confirm operator key via secret store, multi-RPC list, oracle + window addresses from [`REGISTRY.md`](../../deployments/mainnet-ust1-wrap/REGISTRY.md), `ORACLE_MAX_SILENCE_SECS` (~**28800** / 8h).

3. Confirm `effective_swap.oracle.last_update_sec` advances within policy after deploy.

4. **Silence alert:** pager/channel fires when no successful broadcast within `ORACLE_MAX_SILENCE_SECS`. Attach screenshot/log on [#503](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/503).

---

## Pause playbooks

### A. ust1-window `set_paused`

**Who:** wrap-stack governance `terra1xsecn4…` (**O6** — not DEX multisig).

```bash
export WINDOW_ADDR=terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2
export CHAIN_ID=columbus-5
export NODE=https://terra-classic-rpc.publicnode.com:443

terrad tx wasm execute "$WINDOW_ADDR" '{"set_paused":{"paused":true}}' \
  --from "$GOVERNANCE_KEY" --chain-id "$CHAIN_ID" --node "$NODE" \
  --gas auto --gas-adjustment 1.4 --fees 500000uluna -y

terrad tx wasm execute "$WINDOW_ADDR" '{"set_paused":{"paused":false}}' \
  --from "$GOVERNANCE_KEY" --chain-id "$CHAIN_ID" --node "$NODE" \
  --gas auto --gas-adjustment 1.4 --fees 500000uluna -y
```

Confirm execute JSON against ust1-window schema if message shape drifts. Record tx hashes on #503.

### B. wrap-mapper `set_paused`

See [`wrap-mapper-pause.md`](./wrap-mapper-pause.md). LocalTerra: `make smoke-wrap-mapper-pause` ([#396](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/396)).

### C. Treasury `set_wrapping_paused`

Secondary control on the CMM treasury (ustr-cmm). Use when wrap-mapper pause is insufficient or treasury-side wrapping must stop.

```bash
export TREASURY_ADDR=terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2

# Confirm message shape against ustr-cmm schema before broadcast
terrad tx wasm execute "$TREASURY_ADDR" '{"set_wrapping_paused":{"paused":true}}' \
  --from "$GOVERNANCE_KEY" --chain-id columbus-5 \
  --node https://terra-classic-rpc.publicnode.com:443 \
  --gas auto --gas-adjustment 1.4 --fees 500000uluna -y

terrad tx wasm execute "$TREASURY_ADDR" '{"set_wrapping_paused":{"paused":false}}' \
  --from "$GOVERNANCE_KEY" --chain-id columbus-5 \
  --node https://terra-classic-rpc.publicnode.com:443 \
  --gas auto --gas-adjustment 1.4 --fees 500000uluna -y
```

### D. Rate-limit vs indexer 429

| Signal | Layer | Action |
|--------|-------|--------|
| Wrap/unwrap “rate limit” | On-chain wrap-mapper | Wait / smaller amount / governance raise caps |
| HTTP 429 | Indexer | Retry-After; not a wrap pause |

---

## Launch go/no-go extras (UST1 / wrap)

- [ ] Coolify matches [`coolify.env.example`](../../deployments/mainnet-ust1-wrap/coolify.env.example)
- [ ] Strict health probe green (or accepted WARN with #503 comment)
- [ ] Oracle silence alert evidence on #503
- [ ] Treasury vFDUSD capacity + wrap solvency OK
- [ ] Wrap-mapper unpaused; pause drill notes attached (or dry-run)
- [ ] Secondary AMM Path A seeded **or** Path B waiver still accepted ([#508](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/508))

Pair with [`launch-checklist.md` Phase 5](./launch-checklist.md) ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)).

---

## User-facing copy

| Incident | FAQ |
|----------|-----|
| UST1 window / oracle paused or stale | [user-incident-faq.md § UST1](../user-incident-faq.md#ust1-oracle-window) |
| Wrap pause / rate limit | [user-incident-faq.md § Wrap pause](../user-incident-faq.md#wrap-pause) |

---

## Verification

```bash
make verify-issue-503
VERIFY503_MAINNET=1 make verify-issue-503
```

## Related

- [`wrap-mapper-pause.md`](./wrap-mapper-pause.md), [`ust1-window-ui.md`](./ust1-window-ui.md), [`mainnet-soft-launch.md`](./mainnet-soft-launch.md)
- [`launch-monitoring.md`](./launch-monitoring.md), [`emergency-commands.md`](./emergency-commands.md)
- QA: [`wrap-unwrap-test-pass.md`](../qa-templates/wrap-unwrap-test-pass.md)
