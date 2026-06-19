# Agent playbook: IBC-hooks deploy gate (SEC-D02)

Use when verifying or extending **pre-launch / deploy runbook** coverage for Terra Classic **IBC-hooks chain exposure** and whether **CL8Y DEX contracts** implement IBC CosmWasm callbacks ([#407](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/407)).

## Threat model (short)

IBC-hooks middleware on a chain can route ICS-20 transfers into CosmWasm `ibc_receive` (and related ack/timeout) paths. Bugs in that stack have historically affected bridged and CW20 assets on peer networks. CL8Y DEX **pool-only** contracts do not register IBC entry points today, but operators must **record** chain IBC-hooks status at deploy time and **re-check** after chain upgrades or when adding new wasm modules.

## Canonical references

| Doc / script | Purpose |
|--------------|---------|
| [docs/runbooks/launch-checklist.md](../docs/runbooks/launch-checklist.md) Phase 0 | **Mandatory** deploy gate — chain version, IBC-hooks probe, operator attestation |
| [docs/deployment-guide.md](../docs/deployment-guide.md) Post-Deployment Checklist | Same gate after contract upload |
| [docs/security-model.md § IBC hooks](../docs/security-model.md#ibc-hooks-chain-dependency-sec-d02) | Security narrative + cross-links |
| `scripts/verify-no-ibc-hooks-in-contracts.sh` | Static grep — no `ibc_receive` / `ibc_ack` / `ibc_timeout` in `smartcontracts/contracts/` |
| `scripts/lib/record-chain-ibc-hooks-version.sh` | Optional LCD helper to paste into launch issue |
| `scripts/check_ibc_hooks_deploy_docs.py` | Drift guard for SEC-D02 doc markers |

## Operator workflow

1. **Before mainnet deploy (Phase 0):** run chain probes against production LCD; paste output on the launch tracking issue.
2. **Static contract check:** `make verify-no-ibc-hooks-in-contracts` (must pass).
3. **Attestation:** post the required sentence from the launch runbook (commit SHA + UTC date).
4. **After chain upgrade or new contract crate:** repeat steps 1–3 before the next production release.

```bash
# Doc + static contract posture (no LocalTerra required)
make verify-issue-407

# Chain record helper (set production LCD)
TERRA_LCD_URL=https://lcd.terra.dev ./scripts/lib/record-chain-ibc-hooks-version.sh
```

## Rules of thumb

1. **Do not** conflate **after-swap hooks** (pair `AfterSwap` policy) with **IBC packet hooks** — different surfaces; both are documented in [security-model.md](../docs/security-model.md).
2. If a future feature adds `ibc_receive` / `ibc_ack` / `ibc_timeout`, update the runbook attestation, threat model, and obtain security sign-off **before** mainnet upload.
3. **Re-verification** is required on chain upgrades even when contract code is unchanged — the chain dependency can change independently.

## Related

- Launch go/no-go: [`AGENTS_LAUNCH_GO_NO_GO.md`](./AGENTS_LAUNCH_GO_NO_GO.md)
- After-swap hook registration: [`docs/runbooks/hook-registration.md`](../docs/runbooks/hook-registration.md)
- CW20 / bridged asset policy: [`docs/runbooks/cw20-whitelist-policy.md`](../docs/runbooks/cw20-whitelist-policy.md)
