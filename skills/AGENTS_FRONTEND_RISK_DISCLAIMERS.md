# Agent playbook: frontend risk disclaimers and first-visit acknowledgement

Use when changing **NFA / risk copy**, **`RiskAcknowledgementModal`**, **`EnvironmentRibbon`**, **`riskAcknowledgement` storage**, or **Playwright env** around the first-visit gate ([GitLab #138](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Risk surfacing, NFA copy, and first-visit acknowledgement](../docs/frontend.md#legal-risk-surfacing) | Invariants, `VITE_PLAYWRIGHT_E2E`, storage key / version |
| `frontend-dapp/src/utils/riskAcknowledgement.ts` | `localStorage` persistence and `RISK_ACK_VERSION` |
| `frontend-dapp/src/components/legal/legalCopy.ts` | Single source for bullets and NFA strings shared by modal + footer |
| `frontend-dapp/src/components/ui/Modal.tsx` | `dismissible` prop for blocking dialogs |
| `frontend-dapp/playwright.config.ts` | Sets `VITE_PLAYWRIGHT_E2E=true` on `webServer` so E2E is not blocked |

## Rules of thumb

1. **Do not** make the first-visit risk modal closable via backdrop or Escape without product/legal sign-off; keep **`dismissible={false}`** on that use case.
2. **Bump `RISK_ACK_VERSION`** when changing disclaimer meaningfully so returning browsers re-run the gate.
3. **Keep `VITE_PLAYWRIGHT_E2E` off** for real builds and manual QA; it exists only to unblock Playwright against the same dev server command.
4. **Edit copy in one place** (`legalCopy.ts`) unless you intentionally split mobile vs desktop tone. **Mobile footer:** when the shell hides the desktop footer (`max-width: 767px`), `Layout` mounts a second `LegalFooterNotice` in `.app-mobile-legal-strip` so NFA copy still appears on phones.

## Related

- **Responsive shell / header:** [`docs/frontend.md` § Responsive shell & header navigation](../docs/frontend.md#responsive-header-navigation), [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md) ([GitLab #136](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136)).
- **Production build / maps:** [`AGENTS_FRONTEND_PRODUCTION_BUILD.md`](./AGENTS_FRONTEND_PRODUCTION_BUILD.md).
