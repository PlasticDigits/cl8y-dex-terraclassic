# Agent playbook: CL8Y Legal clickwrap (Terra Classic TermsGate)

Use when changing **connected-wallet Legal gating**, **Legal next-step wallet hint copy**, **`@plasticdigits/cl8y-clickwrap`**, **CSP Legal hosts**, **portal redirect allowlist helpers**, or **Playwright escape hatches** for terms acceptance ([GitLab #517](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/517), [#658](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/658)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § CL8Y Legal clickwrap](../docs/frontend.md#legal-clickwrap) | Invariants **C1–C10**, env vars, ops allowlists |
| `frontend-dapp/src/utils/legalClickwrap.ts` | Property, client singleton, redirect sanitize, automation skip |
| `frontend-dapp/src/components/legal/ConnectedTermsGate.tsx` | Shell gate around `<Outlet>` in `Layout.tsx` |
| `frontend-dapp/src/utils/legalKeplrInAppHint.ts` | Legal next-step hint copy + hide rule (**WC-M12** / **L658**) |
| `frontend-dapp/viteCsp.ts` | Production `connect-src` includes Legal API + portal origins |
| `frontend-dapp/.npmrc` | `@plasticdigits` → GitLab npm package registry (Legal SDK) |
| `frontend-dapp/playwright.config.ts` | `VITE_PLAYWRIGHT_E2E=true` skips risk ack **and** clickwrap |
| First-visit NFA (separate) | [`AGENTS_FRONTEND_RISK_DISCLAIMERS.md`](./AGENTS_FRONTEND_RISK_DISCLAIMERS.md) (#138) |
| External Legal platform | [`cl8y-ecosystem-legal`](https://gitlab.com/PlasticDigits/cl8y-ecosystem-legal) SDK `packages/cl8y-clickwrap` |

## Invariants (C1–C10)

1. **C1 — SDK only:** Use `@plasticdigits/cl8y-clickwrap` (`TermsGate` / `createClient`). **Do not** implement Terra Classic ADR-036 verify or wallet submit in the DEX. After WalletConnect without a keplr-like signer injector, surface the DEX-wallet Legal hint (**not Keplr-only**; [#658](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/658) **WC-M12**) — do not claim connect is complete if Accept still needs an injected signer on the Legal portal.
2. **C2 — Property:** Frontend status + portal sign URLs use property **`dex.cl8y.com`** (override only via `VITE_LEGAL_PROPERTY` for staging). Do not check `cl8y.com` by accident.
3. **C3 — Network:** Always `network="TerraClassic"` → API `TERRA_CLASSIC`. Never EVM/Solana/Telegram on this dapp.
4. **C4 — Sequence:** Keep anonymous first-visit [`RiskAcknowledgementModal`](../frontend-dapp/src/components/legal/RiskAcknowledgementModal.tsx) (#138). Clickwrap runs **after connect**, not as a replacement for NFA/footer copy.
5. **C5 — Fail closed:** After connect, unknown/error Legal status must **not** render transactional route children (swap/LP/limits/wrap/create). Browse while **disconnected** remains OK.
6. **C6 — Redirect safety:** Pass sanitized `redirect_uri` (`sanitizeRedirectUri` / DEX allowlist including `https://dex.cl8y.com`). Portal allowlist remains authoritative.
7. **C7 — CSP:** Production `connect-src` must include Legal API (and terms origin) **without** blanket `https:`.
8. **C8 — Secrets:** No `ADMIN_TOKEN` or Legal admin credentials in the frontend — public status/latest endpoints only.
9. **C9 — E2E escape hatch:** `VITE_PLAYWRIGHT_E2E=true` may skip the gate (Playwright `webServer` only). Production / Coolify / manual QA must leave it unset.
10. **C10 — Footer NFA retained:** `LegalFooterNotice` / `EnvironmentRibbon` stay; clickwrap is acceptance evidence, not a substitute for risk disclosure. Official Homepage / Bridge stay in the footer outside this gate ([#663](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/663)).

## Legal wallet hint (L658-1–L658-8)

Retarget **WC-M12** so Station / Cosmostation / Lunc Dash / Galaxy Station users are not told Keplr is required ([#658](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/658)). Portal wallet matrix stays in `cl8y-ecosystem-legal` (Leap on the portal is **not** a DEX Connect wallet — [#159](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/159)).

1. **L658-1 — Show when unsigned + no injector:** `signedLatest === false` and no keplr-like injector → hint visible. Copy must **not** say Keplr is the only accepted wallet.
2. **L658-2 — Name DEX wallets or the connected wallet:** Multi-wallet sentence lists Station, Keplr, Cosmostation, Lunc Dash, Galaxy Station **or** a one-wallet sentence for the connected `walletType`. **Leap** is never named on the DEX.
3. **L658-3 — Hide on `window.keplr`:** Unsigned + Keplr injector → hint hidden (existing #554 behavior).
4. **L658-4 — Hide on Station shim:** Unsigned + `window.station.keplr` (even without `window.keplr`) → hint hidden.
5. **L658-5 — Hide on Cosmostation provider:** Unsigned + `window.cosmostation.providers.keplr` → hint hidden.
6. **L658-6 — Hide when signed or unknown:** `signed_latest === true` or status `null` / loading / error → hint hidden. Fail-closed still blocks transactional children (**C5**).
7. **L658-7 — Station without shim still shows:** `'station' in window` with **no** `station.keplr` is not a signer injector — keep the hint.
8. **L658-8 — Hint is copy only:** Plain text (no wallet-download `<a href>`). Accept still navigates to `terms.cl8y.com/sign/terra-classic?property=dex.cl8y.com`. Hide is UX only — a fake `window.keplr = {}` cannot produce `signed_latest`.

Detection reuses [`getKeplrLikeExtension`](../frontend-dapp/src/services/terraclassic/keplrLikeExtension.ts) via `hasLegalSignerInjector`. Do not treat `isBrowserWalletExtensionDetected(Station)` (`'station' in window`) as enough.

## Shell vs CTA block

**Chosen:** shell `ConnectedTermsGate` wrapping `<Outlet>` in [`Layout.tsx`](../frontend-dapp/src/components/common/Layout.tsx) so header/wallet disconnect stay available while main content is gated. Prefer this over per-page CTA duplication unless product requires browse-while-connected-unsigned.

## Ops (Legal platform — often separate MR)

These are **not** enforced by DEX unit tests; document completion on the issue/MR:

1. Register property `dex.cl8y.com` on Legal admin API using the Legal repo script (interactive hidden token — **not** `ADMIN_TOKEN` env):

   ```bash
   # from cl8y-ecosystem-legal checkout
   ./scripts/register-property.sh dex.cl8y.com "CL8Y DEX"
   ./scripts/register-property.sh --list
   ```

   Requires Legal API deploy with `POST /admin/properties` (see Legal `skills/security-ops/SKILL.md`).
2. Coolify Legal API `CORS_ORIGINS` includes `https://dex.cl8y.com` (localhost only if needed for local).
3. Rebuild Legal portal with `VITE_REDIRECT_URI_ALLOWLIST` including `https://dex.cl8y.com`.

## npm install notes

`@plasticdigits/cl8y-clickwrap` is published to the **GitLab** npm registry for `cl8y-ecosystem-legal` (project id `82547916`). `frontend-dapp/.npmrc` scopes `@plasticdigits` there. Public project packages install without auth.

```bash
cd frontend-dapp && npm install @plasticdigits/cl8y-clickwrap
```

Do **not** replace the GitLab tarball with a local pass-through stub (`TermsGate` always returns `children`, `isAllowedRedirectUri` always `true`). That skips the connected gate and allowlists attacker redirect origins. `make verify-issue-658` fails closed if the installed package is stubbed. Worktree `node_modules` symlinks inherit a stub from the primary checkout — reinstall the package (or `npm ci`) before #658 / #673.

## Rules of thumb

1. **Do not** treat `cl8y-dex-risk-ack` localStorage as Legal signature proof.
2. **Do not** embed the portal in an iframe for Accept — full navigation to `sign_urls.terra_classic`.
3. **Do not** widen CSP `connect-src` to `https:` to “fix” Legal.
4. Keep `VITE_PLAYWRIGHT_E2E` off for soft-launch manual QA on `https://dex.cl8y.com`.
5. When Legal terms version bumps, rely on SDK focus re-poll / status — do not cache `signed_latest` forever in localStorage.
6. Do **not** tell users Keplr is required to accept terms. Hide the hint when any keplr-like injector is present; never name Leap on the DEX ([#658](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/658) **L658**).

## Verification

```bash
make verify-issue-517
make verify-issue-658
make verify-issue-554
# or:
bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
  src/utils/__tests__/legalClickwrap.test.ts \
  src/utils/__tests__/legalKeplrInAppHint.test.ts \
  src/components/legal/__tests__/ConnectedTermsGate.test.tsx \
  src/utils/__tests__/viteCsp.test.ts
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/legal-clickwrap-517.spec.ts --project=e2e-smoke
```

Manual soft-launch: connect unsigned wallet → portal Terra Classic sign → return → `signed_latest: true` → one swap/LP action.

## Related

- Risk / NFA first-visit: [`AGENTS_FRONTEND_RISK_DISCLAIMERS.md`](./AGENTS_FRONTEND_RISK_DISCLAIMERS.md)
- Official CL8Y product links (footer Homepage / Bridge): [`AGENTS_FRONTEND_PRODUCT_LINKS.md`](./AGENTS_FRONTEND_PRODUCT_LINKS.md) ([#663](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/663))
- Connect Wallet dismiss without connecting must not skip this gate ([#672](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/672) **D8**): [`AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md`](./AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md)
- Trust / CSP: [`AGENTS_FRONTEND_TRUST_BOUNDARIES.md`](./AGENTS_FRONTEND_TRUST_BOUNDARIES.md)
- Soft-launch: [`docs/runbooks/mainnet-soft-launch.md`](../docs/runbooks/mainnet-soft-launch.md)
- WalletConnect same-device mobile pairing / Legal hint: [`AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md`](./AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md) (**WC-M12**, [#554](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/554) / [#658](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/658))
- Retail copy: [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) (#489 — one short sentence, no always-on essay after `signed_latest`)
- Post-merge leftover: [#673](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/673) / `make verify-issue-673` / [`AGENTS_POST_MERGE_OPS_673.md`](./AGENTS_POST_MERGE_OPS_673.md)
