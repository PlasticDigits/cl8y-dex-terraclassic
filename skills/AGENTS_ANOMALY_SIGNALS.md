# Agent playbook: proactive anomaly signals (SEC-G02)

Use when changing **operator anomaly thresholds**, **incident triage checklists**, or **small-TVL monitoring guidance** — not user-facing FAQ copy ([GitLab **#435**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/435)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/runbooks/anomaly-signals.md](../docs/runbooks/anomaly-signals.md) | **Single** proactive anomaly checklist (thresholds + first response) |
| [docs/templates/incident-dex-indexer.md](../docs/templates/incident-dex-indexer.md) | Incident tracker — Triage links to anomaly runbook |
| [docs/runbooks/blacklist-decision.md](../docs/runbooks/blacklist-decision.md) | **Reactive** blacklist after confirmed evidence |
| [docs/security-posture.md](../docs/security-posture.md) | Bootstrap TVL band — monitoring row links to anomaly runbook |
| `scripts/check_anomaly_signals_docs.py` | Drift guard for SEC-G02 acceptance topics + links |

## Rules of thumb

1. **Proactive ≠ blacklist:** anomaly signals trigger investigation, pair pause, or rate-limit review — not factory blacklist without [blacklist-decision.md](../docs/runbooks/blacklist-decision.md) evidence.
2. **Do not** duplicate the full checklist in `security-posture.md` or the incident template — **link** to `docs/runbooks/anomaly-signals.md`.
3. **Threshold edits** must keep all five signals (A1–A5): pool drain %, add/remove loop, route slippage deviation, failed tx burst, LCD-heavy 429 flood — each with a numeric threshold and first response.
4. **Slippage alignment:** A3 **30%** route slippage band matches retail Expert Mode guard ([#293](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/293)); operator threshold is for **monitoring**, not dApp UX.
5. **LCD-heavy paths:** A5 references the same route list as [AGENTS_INDEXER_API_LCD_SECURITY.md](./AGENTS_INDEXER_API_LCD_SECURITY.md).

## Verification

```bash
make check-anomaly-signals-docs
make verify-issue-435
```

## Related

- Blacklist (reactive): [`AGENTS_BLACKLIST_DECISION.md`](./AGENTS_BLACKLIST_DECISION.md)
- Emergency on-chain commands: [`AGENTS_EMERGENCY_COMMANDS.md`](./AGENTS_EMERGENCY_COMMANDS.md)
- Indexer rate limits: [`AGENTS_INDEXER_API_LCD_SECURITY.md`](./AGENTS_INDEXER_API_LCD_SECURITY.md)
- Launch go/no-go: [`AGENTS_LAUNCH_GO_NO_GO.md`](./AGENTS_LAUNCH_GO_NO_GO.md)
