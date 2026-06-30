# Agent playbook: incident communications templates (SEC-G05)

Use when adding or verifying **paste-ready operator communications** for pause, blacklist, exploit interim, false-alarm retraction, and postmortem scenarios ([GitLab **#438**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/438)).

## Canonical references

| Doc | Purpose |
|-----|---------|
| [docs/templates/incident-dex-indexer.md](../docs/templates/incident-dex-indexer.md) | Incident tracker + **Appendix: Communications templates (SEC-G05)** |
| [docs/user-incident-faq.md](../docs/user-incident-faq.md) | Trader/LP **background** — link, do not duplicate in per-incident copy |
| [docs/runbooks/emergency-commands.md](../docs/runbooks/emergency-commands.md) | On-chain pause/blacklist commands |
| [docs/runbooks/blacklist-decision.md](../docs/runbooks/blacklist-decision.md) | Blacklist criteria + false-positive rollback before retraction comms |
| `scripts/check_incident_comms_templates_docs.py` | Drift guard for SEC-G05 acceptance topics + cross-links |

## Five required templates

| # | Scenario | Public use |
|---|----------|------------|
| 1 | Pair paused | User-facing pause announcement |
| 2 | Blacklist applied | Compliance / incident restriction notice |
| 3 | Exploit under investigation | Interim notice while scope unknown |
| 4 | False alarm retraction | After wrongful pause or blacklist |
| 5 | Postmortem summary | Closed incident summary + follow-ups |

Each template must include fill-in placeholders for: **address** (pair/wallet/token), **impact**, **timestamp**, **estimated resolution or completed actions**, and **contact channel**.

## Rules of thumb

1. **Do not** put per-incident announcement copy in `user-incident-faq.md` — that doc is static background; templates live in the incident template appendix.
2. **Do not** publish exploit recipes, unconfirmed attacker addresses, or PoC details in public templates.
3. **False alarm:** complete the [rollback checklist](../docs/runbooks/blacklist-decision.md#false-positive-rollback-unblacklist) before sending template 4.
4. **Funds messaging:** controls gate execute paths; they do **not** confiscate wallet balances (align with [AGENTS_USER_INCIDENT_FAQ.md](./AGENTS_USER_INCIDENT_FAQ.md)).

## Verification

```bash
make check-incident-comms-templates-docs
bash scripts/qa/verify-issue-438.sh
```

## Related

- Emergency on-chain commands: [`AGENTS_EMERGENCY_COMMANDS.md`](./AGENTS_EMERGENCY_COMMANDS.md)
- Blacklist decision tree: [`AGENTS_BLACKLIST_DECISION.md`](./AGENTS_BLACKLIST_DECISION.md)
- User incident FAQ (retail copy): [`AGENTS_USER_INCIDENT_FAQ.md`](./AGENTS_USER_INCIDENT_FAQ.md)
