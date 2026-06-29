# Agent playbook: user incident FAQ (SEC-A03)

Use when changing **user-facing copy** for **pause**, **trading blacklist**, or **rate-limit** states, or when adding links from security docs / the legal footer ([GitLab **#390**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/390)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/user-incident-faq.md](../docs/user-incident-faq.md) | **Single** plain-language incident guide for traders and LPs |
| [docs/security-model.md](../docs/security-model.md) | Security entry point — must link to the FAQ |
| [docs/contracts-security-audit.md](../docs/contracts-security-audit.md) | Invariant **L6** (pause), blacklist guards — technical, not user copy |
| [docs/adr/0003-governance-trading-blacklist.md](../docs/adr/0003-governance-trading-blacklist.md) | Blacklist design (admin) |
| [docs/runbooks/blacklist-decision.md](../docs/runbooks/blacklist-decision.md) | Operator decision tree + rollback (SEC-B12) |
| `frontend-dapp/src/components/legal/legalCopy.ts` | `USER_INCIDENT_FAQ_HREF` / `USER_INCIDENT_FAQ_LABEL` |
| `frontend-dapp/src/components/legal/LegalFooterNotice.tsx` | Footer link to FAQ |
| `frontend-dapp/src/services/terraclassic/blacklist.ts` | `describeTradingBlacklistBlock` inline strings |
| `scripts/check_user_incident_faq_docs.py` | Drift guard for SEC-A03 acceptance topics + links |

## Rules of thumb

1. **Do not** duplicate the full FAQ in `security-model.md`, runbooks, or UI — **link** to `docs/user-incident-faq.md`.
2. **Pause vs blacklist:** pause is per-pair governance; blacklist can target wallet, token, or pair on the factory ([ADR 0003](../docs/adr/0003-governance-trading-blacklist.md)).
3. **Funds messaging:** controls gate execute paths; they do **not** burn wallet balances. Escrow and LP stay in contracts until normal withdraw/cancel/claim after lift.
4. **Rate limits:** distinguish **indexer HTTP 429** (off-chain, retry) from **wrap-mapper** on-chain caps (native wrap/unwrap). Frontend regression: **SEC-E04** / GitLab **#426** — `isIndexerRateLimitError`, `INDEXER_RATE_LIMIT_RETRY_MESSAGE`, Vitest in `indexerErrors.test.ts`, `humanizeUserFacingError.test.ts`, `SwapPage.test.tsx`.
5. **Tier 255 ≠ trading blacklist:** tier 255 only removes fee discounts; factory blacklist stops trading.

## Verification

```bash
make check-user-incident-faq-docs
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- src/utils/__tests__/indexerErrors.test.ts src/utils/__tests__/humanizeUserFacingError.test.ts src/pages/SwapPage.test.tsx -t "429|SEC-E04"
```

After UI copy changes, run targeted frontend unit tests if you touch `LegalFooterNotice` or pause/blacklist banners:

```bash
cd frontend-dapp && npm run test:unit -- src/components/legal/LegalFooterNotice.test.tsx
```

## Related

- Operator blacklist criteria: [`AGENTS_BLACKLIST_DECISION.md`](./AGENTS_BLACKLIST_DECISION.md)
- Risk disclaimers / NFA footer: [`AGENTS_FRONTEND_RISK_DISCLAIMERS.md`](./AGENTS_FRONTEND_RISK_DISCLAIMERS.md)
- Indexer API rate limits: [`AGENTS_INDEXER_API_LCD_SECURITY.md`](./AGENTS_INDEXER_API_LCD_SECURITY.md)
- Limit order pause / parked expiry: [`AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md`](./AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md)
