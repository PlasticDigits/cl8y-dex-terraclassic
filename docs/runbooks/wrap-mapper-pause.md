# Wrap-mapper pause playbook (columbus-5)

Step-by-step **pause / unpause** for the ustr-cmm **wrap-mapper** used by CL8Y DEX native wrap ([GitLab **#503**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/503), Phase 3 [#507](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/507), parent [#502](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/502)).

**Related:** ops hub [`ust1-wrap-production-ops.md`](./ust1-wrap-production-ops.md), LocalTerra smoke [`scripts/smoke-wrap-mapper-pause.sh`](../../scripts/smoke-wrap-mapper-pause.sh) (`make smoke-wrap-mapper-pause`, SEC-B06 / [#396](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/396)), user FAQ [§ Wrap pause](../user-incident-faq.md#wrap-pause), QA template [`wrap-unwrap-test-pass.md`](../qa-templates/wrap-unwrap-test-pass.md).

---

## Authority

- **Signer:** wrap-mapper config.governance. At columbus-5 block **30540667** (2026-09-24 09:09 UTC), config.governance and ContractInfo.admin are the DEX 2-of-3 terra1zlmv2…hep7 ([#525](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/525), [#526](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/526), [#638](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/638)). The treasury uses the same multisig; see [REGISTRY.md](../../deployments/mainnet-ust1-wrap/REGISTRY.md#governance-split-ops-critical).
- Before a transaction, query wrap-mapper config and LCD ContractInfo.admin separately. The health probe does not attest wasm admin; see [#697](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/697).
- Addresses: [`deployments/mainnet-ust1-wrap/REGISTRY.md`](../../deployments/mainnet-ust1-wrap/REGISTRY.md).

| Role | Address |
|------|---------|
| wrap-mapper | `terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2` |
| CMM treasury | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` |

---

## When to pause

- Suspected wrap insolvency (treasury native < CW20 supply) — **O5**
- Mapper / treasury bug, malicious mint path, or key incident
- Controlled rehearsal (off-peak) with unpause planned in the same window

Do **not** use factory pair pause as a substitute — that does not stop treasury `wrap_deposit` / CW20 unwrap.

---

## Read-only preflight

```bash
./scripts/check-ust1-wrap-ops-health.sh
# Expect wrap-mapper not paused before a deliberate pause drill
```

Or LCD:

```bash
# config.paused should be false before drill
# Use scripts/lib/lcd-smart-query.sh helpers or terrad query wasm contract-state smart
```

---

## Pause (mainnet)

```bash
export WRAP_MAPPER=terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2
export TERRAD_HOST_CHAIN_ID=columbus-5
export TERRAD_HOST_NODE=https://terra-classic-rpc.publicnode.com:443
# Human multisig signers run this helper; it generates, signs with two keys, and broadcasts.
./scripts/multisig-2of3-host-tx.sh wasm execute "$WRAP_MAPPER" '{"set_paused":{"paused":true}}'
```

**Verify:**

1. `config.paused == true`
2. Small wrap / unwrap fails with paused error
3. UI on Swap / Wrap shows **Wrapping is Temporarily Paused** (SEC-A02)
4. Non-wrap CW20 swaps still work

---

## Unpause

```bash
# Human multisig signers only.
./scripts/multisig-2of3-host-tx.sh wasm execute "$WRAP_MAPPER" '{"set_paused":{"paused":false}}'
```

**Verify:** wrap + unwrap succeed; UI CTA cleared; `./scripts/check-ust1-wrap-ops-health.sh` shows not paused.

---

## Columbus-5 smoke procedure (record on #503)

LocalTerra proof is **not** a substitute for mainnet rehearsal. Prefer **off-peak** and reverse quickly.

| Step | Action | Evidence |
|------|--------|----------|
| 1 | Preflight health script | log paste |
| 2 | `set_paused: true` | tx hash |
| 3 | Failed wrap + failed unwrap | tx hashes / error strings |
| 4 | Confirm UI pause CTA | screenshot optional |
| 5 | `set_paused: false` | tx hash |
| 6 | Successful wrap + unwrap (tiny amounts) | tx hashes |
| 7 | Comment on [#503](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/503) | drill notes |

If governance keys are unavailable, record a **dry-run** on [#503]: read-only `paused=false`, LocalTerra `make smoke-wrap-mapper-pause` transcript, and scheduled date for live drill.

### LocalTerra (rehearsal without mainnet keys)

```bash
make deploy-local   # full wrap seed
make smoke-wrap-mapper-pause
# or: make verify-issue-396
```

---

## Distinctions

| Control | Effect |
|---------|--------|
| wrap-mapper `set_paused` | Blocks wrap + unwrap via mapper |
| Factory pair `SetPaused` | Blocks that pool’s swaps/LP/limits — not native wrap |
| Wrap-mapper `rate_limit` | Per-denom cap (read/raise in [`ust1-wrap-production-ops.md`](./ust1-wrap-production-ops.md) § D) |
| Indexer HTTP 429 | Off-chain only |

---

## Related

- [`ust1-wrap-production-ops.md`](./ust1-wrap-production-ops.md) invariants **O3**, **O5**
- [`skills/AGENTS_FRONTEND_SWAP_SAFETY_CTA.md`](../../skills/AGENTS_FRONTEND_SWAP_SAFETY_CTA.md)
- [`NATIVE_TOKEN_WRAPPING.md`](../../NATIVE_TOKEN_WRAPPING.md)
