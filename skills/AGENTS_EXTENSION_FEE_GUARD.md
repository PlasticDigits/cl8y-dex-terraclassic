# Agent playbook: extension fee guard scope (SEC-E08)

Use when reviewing **post-sign fee/gas guards** for extension wallets, launch checklist wallet QA, or questions about why **`extensionSignedFeeGuard`** is **LocalTerra-only** ([#429](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/429)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/security-model.md § Extension fee guard](../docs/security-model.md#extension-wallet-fee-guard-sec-e08) | Security narrative — LocalTerra-only scope, mainnet exclusion |
| [docs/runbooks/extension-fee-guard-wallet-qa.md](../docs/runbooks/extension-fee-guard-wallet-qa.md) | Manual wallet QA steps + launch sign-off text |
| [docs/runbooks/launch-checklist.md](../docs/runbooks/launch-checklist.md) Phase 4 | **SEC-E08** launch gate |
| [`extensionSignedFeeGuard.ts`](../frontend-dapp/src/utils/extensionSignedFeeGuard.ts) | Guard logic; `isLocalTerraChainId` gate |
| [`extensionSignedFeeGuard.test.ts`](../frontend-dapp/src/utils/__tests__/extensionSignedFeeGuard.test.ts) | Unit tests — mainnet `columbus-5` returns `null` |
| [GitLab #127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127) | LocalTerra Station/Keplr stale-fee repro |
| [GitLab #134](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/134) | Partial fee rewrite (~23 vs ~36 LUNC) |
| [GitLab #371](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/371) | Retail UI copy for guard failures |
| [`AGENTS_FRONTEND_STATION_SIGNING.md`](./AGENTS_FRONTEND_STATION_SIGNING.md) | Station shim, amino signing, LocalTerra wallet matrix |
| [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md) | Fee envelopes, cosmes patch, gas constants |

## Rules of thumb

1. **LocalTerra only** — `extensionSignedFeeUndershootMessage` returns `null` for non-`localterra` chain IDs (e.g. `columbus-5`). This is **intentional**, not a mainnet gap.
2. **Keplr on mainnet** — does **not** exhibit the stale-fee rewrite that prompted the guard (maintainer confirmation [#429](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/429)). Production relies on cosmes `preferNoSetFee` + dApp `Fee` envelope without a post-sign check.
3. **Do not extend to mainnet** without a new repro on production networks and security sign-off.
4. **LocalTerra QA** — use **Keplr** or **simulated wallet**, not Terra Station ([#235](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/235)).
5. **Keep patch in sync** — guard logic is duplicated in `patches/@goblinhunt+cosmes+*.patch`; run `npm ci` and `cosmesPatch127.test.ts` after patch edits ([#367](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/367)).

## Verification

```bash
# Doc invariants + unit tests (no LocalTerra required)
make verify-issue-429
```

Manual wallet QA: [extension-fee-guard-wallet-qa.md](../docs/runbooks/extension-fee-guard-wallet-qa.md).

## Related

- User-facing errors: [`AGENTS_FRONTEND_USER_ERRORS.md`](./AGENTS_FRONTEND_USER_ERRORS.md)
- Launch go/no-go: [`AGENTS_LAUNCH_GO_NO_GO.md`](./AGENTS_LAUNCH_GO_NO_GO.md)
