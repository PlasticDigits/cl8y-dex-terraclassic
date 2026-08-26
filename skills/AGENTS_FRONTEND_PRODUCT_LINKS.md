# Agent playbook: official CL8Y product links (footer)

Use when changing the **shell footer product row**, **Homepage / Bridge outbound links**, or **header More / Swap banners** that might grow an external CL8Y URL ([GitLab **#663**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/663)).

This DEX does **not** ship the Bridge. Link only. Do **not** iframe `bridge.cl8y.com`. Do **not** add GameFi / Telegram / X / CEX tiles in this chrome.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Official CL8Y product links](../docs/frontend.md#official-cl8y-product-links) | Invariants **P663-1–P663-8** |
| [`cl8yProductLinks.ts`](../frontend-dapp/src/utils/cl8yProductLinks.ts) | Frozen HTTPS allowlist + `isAllowedCl8yProductHref` |
| [`Cl8yProductLinks.tsx`](../frontend-dapp/src/components/common/Cl8yProductLinks.tsx) | Footer `nav` (`aria-label="CL8Y products"`) |
| [`Layout.tsx`](../frontend-dapp/src/components/common/Layout.tsx) | Mounts the row **inside** `footer.app-footer-shell`, **outside** `ConnectedTermsGate` |
| [`navItems.ts`](../frontend-dapp/src/components/common/navItems.ts) | In-app DEX routes only — **no** external origins |
| [`LegalFooterNotice.tsx`](../frontend-dapp/src/components/legal/LegalFooterNotice.tsx) | NFA / Security / Incidents / LP how-to / Report — keep a **separate** paragraph |
| [`viteCsp.ts`](../frontend-dapp/viteCsp.ts) | Do **not** add apex/bridge to `connect-src` or change `form-action` |

## Invariants (P663-1–P663-8)

1. **P663-1 Footer only** — Homepage (`https://cl8y.com/`) and Bridge (`https://bridge.cl8y.com/`) render in the shell footer. Do **not** add them to header More, mobile More, Swap/Pool banners, or `navItems.ts`.
2. **P663-2 Hardcoded HTTPS allowlist** — hrefs are compile-time constants. No `VITE_CL8Y_*_URL`. Helper rejects `http:`, `//`, `javascript:`, `data:`, userinfo, query/hash, extra path, lookalikes, and `dex.cl8y.com` (already here).
3. **P663-3 New tab + noopener** — each link is `<a target="_blank" rel="noopener noreferrer">`. No `window.open` of user-controlled URLs. No iframe.
4. **P663-4 Separate from legal** — product `nav` is not inside the NFA `<p>`. Labels are **Homepage** and **Bridge** (≤ ~5 words). Not Security / Report / Connect Wallet.
5. **P663-5 LCP + TermsGate** — text only (no footer logo). Product row may paint immediately; `LegalFooterNotice` stays deferred until route-ready (#179). Footer stays mounted while `ConnectedTermsGate` blocks `<Outlet>` (#517).
6. **P663-6 One chrome layer** — no `card-glass` / `shell-panel` inside the footer product row (#653). Links wrap; on 375px they sit **above** `.app-mobile-nav-shell`.
7. **P663-7 Header brand** — logo + **CL8Y DEX** only. Do **not** reintroduce a “Terra Classic ecosystem” kicker (#136).
8. **P663-8 CSP** — linking is navigation, not fetch. Do not widen `connect-src` with blanket `https:` or add apex/bridge hosts. `form-action` stays `'self'`; `frame-ancestors` stays `'none'`.

## Rules of thumb

1. A third official product later **extends the same allowlist + tests**, not a new chrome system.
2. If a property URL moves, update the allowlist, tests, and this playbook in the **same** change.
3. Clickwrap property stays `dex.cl8y.com`. Leaving to homepage/Bridge is not “accept terms here.”
4. Short retail labels only — no whitepaper essay, no “via the canceler network” (#489).

## Regression

```bash
make verify-issue-663
```

Vitest: allowlist helper + `Cl8yProductLinks` + existing `LegalFooterNotice` tests. Playwright: `e2e/footer-product-links-663.spec.ts` (`e2e-smoke`, 5 workers, `PLAYWRIGHT_SKIP_CHAIN=1`). Skip E2E with `VERIFY_ISSUE_663_SKIP_E2E=1`.

## Related

- Footer NFA / ribbon: [`AGENTS_FRONTEND_RISK_DISCLAIMERS.md`](./AGENTS_FRONTEND_RISK_DISCLAIMERS.md) (#138)
- Header density / no ecosystem kicker: [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md) (#136, #483)
- Connected TermsGate (footer stays usable): [`AGENTS_FRONTEND_CLICKWRAP.md`](./AGENTS_FRONTEND_CLICKWRAP.md) (#517)
- Short labels / no lecture banners: [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) (#489)
- No nested footer cards: [`AGENTS_FRONTEND_CHROME_NESTING.md`](./AGENTS_FRONTEND_CHROME_NESTING.md) (#653)
- Production CSP: [`AGENTS_FRONTEND_TRUST_BOUNDARIES.md`](./AGENTS_FRONTEND_TRUST_BOUNDARIES.md)
- Post-merge leftover: [#673](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/673) / `make verify-issue-673` / [`AGENTS_POST_MERGE_OPS_673.md`](./AGENTS_POST_MERGE_OPS_673.md)
