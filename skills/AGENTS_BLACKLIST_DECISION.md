# Agent playbook: trading blacklist decision tree (SEC-B12)

Use when changing **operator criteria** for factory trading blacklist during incidents, compliance / ToS escalations, or false-positive rollback — not user-facing FAQ copy ([GitLab **#400**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/400)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/runbooks/blacklist-decision.md](../docs/runbooks/blacklist-decision.md) | **Single** operator decision tree + rollback checklist |
| [docs/templates/incident-dex-indexer.md](../docs/templates/incident-dex-indexer.md) | Incident tracker template — Mitigation links to runbook; [Communications templates](../docs/templates/incident-dex-indexer.md#appendix-communications-templates-sec-g05) (SEC-G05) |
| [docs/adr/0003-governance-trading-blacklist.md](../docs/adr/0003-governance-trading-blacklist.md) | Blacklist design (on-chain storage, messages) |
| [docs/security-model.md](../docs/security-model.md) | Security entry point — links to runbook + user FAQ |
| [docs/user-incident-faq.md](../docs/user-incident-faq.md) | Trader/LP impact — **link only**, do not duplicate operator criteria |
| `scripts/check_blacklist_decision_docs.py` | Drift guard for SEC-B12 acceptance topics + links |

## Rules of thumb

1. **Do not** duplicate the full decision tree in `security-model.md`, ADR 0003, or the user FAQ — **link** to `docs/runbooks/blacklist-decision.md`.
2. **Suspicion ≠ blacklist:** require confirmed on-chain evidence (exploit actor, malicious token, compromised pair) or documented compliance / ToS case file with sign-off.
3. **Pause vs blacklist:** pair pause is narrower; factory blacklist when scope or bypass requires registry-level gate.
4. **Tier 255 ≠ trading blacklist:** tier 255 only removes fee discounts.
5. **Rollback:** preserve original evidence in the incident record **before** any `Unblacklist*` governance tx.

## Verification

```bash
make check-blacklist-decision-docs
make verify-issue-400
```

## Related

- User incident FAQ (retail copy): [`AGENTS_USER_INCIDENT_FAQ.md`](./AGENTS_USER_INCIDENT_FAQ.md)
- Incident comms templates (SEC-G05): [`AGENTS_INCIDENT_COMMS_TEMPLATES.md`](./AGENTS_INCIDENT_COMMS_TEMPLATES.md)
- Launch governance checklist: [`AGENTS_LAUNCH_GO_NO_GO.md`](./AGENTS_LAUNCH_GO_NO_GO.md)
- Hook blocking (distinct from factory blacklist): [`docs/runbooks/hook-registration.md`](../docs/runbooks/hook-registration.md)
