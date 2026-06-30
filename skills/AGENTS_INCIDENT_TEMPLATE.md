# Agent playbook: incident response template (SEC-G06)

Use when changing the **operator incident tracker** template or verifying timeline audit-trail structure ([GitLab **#439**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/439)).

## Canonical doc

| Path | Purpose |
|------|---------|
| [`docs/templates/incident-dex-indexer.md`](../docs/templates/incident-dex-indexer.md) | Copy-paste incident tracker — metadata, triage, mitigation links, **incident timeline** table |
| [`docs/runbooks/emergency-commands.md`](../docs/runbooks/emergency-commands.md) | Governance tx recipes — record hashes in timeline |
| [`docs/runbooks/blacklist-decision.md`](../docs/runbooks/blacklist-decision.md) | Blacklist rollback checklist — log reversals in timeline |
| [`docs/user-incident-faq.md`](../docs/user-incident-faq.md) | User impact column — link, do not duplicate retail copy |

## Incident timeline (required columns)

Operators add rows as events unfold:

| UTC Time | Tx Hash | Wallet | Token | Pair | Admin Action | User Impact |

Anchor: `docs/templates/incident-dex-indexer.md#incident-timeline`

## Verification

```bash
make check-incident-template-docs
make verify-issue-439
```

## Do not duplicate

- **User-facing** impact copy lives in [`docs/user-incident-faq.md`](../docs/user-incident-faq.md) — summarize in the timeline **User Impact** column and link.
- Emergency command details belong in [`AGENTS_EMERGENCY_COMMANDS.md`](./AGENTS_EMERGENCY_COMMANDS.md); blacklist criteria in [`AGENTS_BLACKLIST_DECISION.md`](./AGENTS_BLACKLIST_DECISION.md).
