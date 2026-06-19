# Agent playbook: user security contact (SEC-A07)

Use when changing **how end users report suspicious trades, balances, or UI states**, or when verifying pre-launch security escalation ([GitLab **#392**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/392)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [`SECURITY.md`](../SECURITY.md) | Root policy — email, GitLab template URL, responsible disclosure, **48–72 hour** acknowledgement |
| [`.gitlab/issue_templates/security_report.md`](../.gitlab/issue_templates/security_report.md) | Public GitLab issue template (`security` label) |
| [`docs/security-model.md`](../docs/security-model.md) § User security contact | Cross-link from threat model |
| [`frontend-dapp/src/components/legal/legalCopy.ts`](../frontend-dapp/src/components/legal/legalCopy.ts) | `SECURITY_CONTACT_EMAIL`, `SECURITY_REPORT_ISSUE_URL`, `SECURITY_RESPONSE_WINDOW` |
| [`frontend-dapp/src/components/legal/LegalFooterNotice.tsx`](../frontend-dapp/src/components/legal/LegalFooterNotice.tsx) | Footer **Report suspicious activity** link |

## Invariants

1. **`SECURITY.md` must stay at repo root** so GitLab and clones surface it in the default file browser.
2. **Contact email** is `contact@ceramicliberty.com` unless product updates the issue — keep `legalCopy.ts` and `SECURITY.md` in sync.
3. **Footer link** must open the GitLab `security_report` template in a new tab (`rel="noopener noreferrer"`).
4. **Responsible disclosure** and **response window** must remain explicit in `SECURITY.md` (no public exploit details before contact; 48–72h acknowledgement).
5. **Do not** route security reports only through internal operator templates (`docs/templates/incident-dex-indexer.md`) — that path is for operators, not retail users.

## Verification checklist (#392)

From repo root:

```bash
test -f SECURITY.md
test -f .gitlab/issue_templates/security_report.md
grep -q '48–72 hours' SECURITY.md
grep -q 'contact@ceramicliberty.com' SECURITY.md
grep -q 'Do not' SECURITY.md
cd frontend-dapp && npm run test:unit -- src/components/legal/__tests__/LegalFooterNotice.test.tsx
```

Manual (with `make dev`): scroll to footer (desktop) or mobile legal strip → **Report suspicious activity** → GitLab new-issue page with security template.

## Related

- Risk disclaimers / NFA footer: [`AGENTS_FRONTEND_RISK_DISCLAIMERS.md`](./AGENTS_FRONTEND_RISK_DISCLAIMERS.md)
- Threat model: [`docs/security-model.md`](../docs/security-model.md)
- QA onboarding security section: [`docs/qa-onboarding.md`](../docs/qa-onboarding.md)
