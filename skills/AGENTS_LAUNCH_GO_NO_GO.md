# Launch go / no-go gate (SEC-A06)

Use this when implementing or verifying **pre-launch sign-off** before production mainnet deploy ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)).

## Canonical runbook

[`docs/runbooks/launch-checklist.md`](../docs/runbooks/launch-checklist.md) — **Phase 5** is the mandatory gate. Phases 0–4 are technical deploy/verify steps on staging; **do not start mainnet Phase 1** until Phase 5 records **GO** or **GO with accepted risk** on the launch tracking issue.

**Phase 1 deploy trace:** Before leaving deploy Phase 1, record git SHA, Terra Classic chain version, code IDs, `wasm-checksums.txt`, and verification output on the launch tracking issue — see [`docs/templates/deploy-trace.md`](../docs/templates/deploy-trace.md) ([#410](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/410), [`AGENTS_DEPLOY_TRACE.md`](./AGENTS_DEPLOY_TRACE.md)).

## Three decisions

| Decision | Meaning |
|----------|---------|
| **BLOCK** | Any P0 category open: admin controls, value-flow invariants, deploy/runbook, user visibility of pause/blacklist/rate-limit risk |
| **PAUSE** | No P0 blockers, but pre-launch gaps without risk acceptance, unrehearsed governance multisig emergency controls (**SEC-B09** — [`governance-emergency-rehearsal.md`](../docs/runbooks/governance-emergency-rehearsal.md)), or incident-runbook gaps |
| **GO / GO with accepted risk** | All P0 closed; pre-launch items closed or explicitly risk-accepted with residual risks documented on the launch issue |

Sign-off format: [`QA_TEMPLATE.md` § SIGN-OFF](../QA_TEMPLATE.md#sign-off) (QA Tester, Dev Lead, Product Owner).

## Regression

```bash
make verify-issue-391
# or doc-only:
make check-launch-go-no-go-docs
```

No LocalTerra or Postgres required.

## Related

- Master executable matrix: [GitLab **#337**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/337) (LR-00)
- Governance emergency rehearsal (SEC-B09): [`AGENTS_GOVERNANCE_EMERGENCY_REHEARSAL.md`](./AGENTS_GOVERNANCE_EMERGENCY_REHEARSAL.md)
- QA onboarding: [`docs/qa-onboarding.md`](../docs/qa-onboarding.md)
- Deployment narrative: [`docs/deployment-guide.md`](../deployment-guide.md)
- Historical blocker matrix (not a live gate): [`docs/reviews/20260409T030009Z/RELEASE_READINESS_MATRIX.md`](../docs/reviews/20260409T030009Z/RELEASE_READINESS_MATRIX.md)
