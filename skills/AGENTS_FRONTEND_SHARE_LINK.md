# Agent playbook: trader profile Share link

Use when adding or changing **in-app Share** of a canonical dApp URL (Web Share + clipboard fallback) — especially `/trader/:address` ([GitLab **#665**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/665)).

This is **not** Open Graph. Crawlers still see the static `index.html` card ([#578](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/578) **OG-5 / OG-6**). Do not add `react-helmet`, prerender, or `og:url` from `window.location` / wallet / pair.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Share link](../docs/frontend.md#share-link-button) | Invariants **TS-1–TS-13** |
| [docs/frontend.md § Trader profile](../docs/frontend.md#trader-profile-indexer) | Share vs AddressRow copy |
| [`sharePageLink.ts`](../frontend-dapp/src/utils/sharePageLink.ts) | Canonical URL + share-vs-copy |
| [`ShareLinkButton.tsx`](../frontend-dapp/src/components/ui/ShareLinkButton.tsx) | Accessible **Share** button |
| [`copyToClipboard.ts`](../frontend-dapp/src/utils/copyToClipboard.ts) | Only clipboard write path |
| [`TraderPage.tsx`](../frontend-dapp/src/pages/TraderPage.tsx) | Required mount (`data-testid="trader-share-link"`) |

## Do / don’t

- **Do** build `{origin}/{kind}/{validatedId}` via `buildCanonicalShareUrl`. Origin is `window.location.origin`. Id must pass `isValidTerraAddress` after trim.
- **Do** prefer `navigator.share` on a user gesture. Treat `AbortError` as cancel (no error live text, no clipboard).
- **Do** fall back to `copyToClipboard` when share is missing, `canShare` is false, or share throws a non-abort error.
- **Do** keep AddressRow **Copy trader address** as bech32-only.
- **Don’t** share `window.location.href` (query/hash / WC URI leak). **Swap exception ([#713](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/713)):** share canonical `/?from=&to=` via `buildCanonicalSwapShareUrl` — still not `location.href`.
- **Don’t** hard-code `https://dex.cl8y.com` or read `VITE_PUBLIC_ORIGIN` in the React bundle for this.
- **Don’t** put P&L, volume, or indexer fields in share `title`/`text`. Static **CL8Y DEX trader** + optional `shortenAddress`. Swap title is **CL8Y DEX swap** plus resolved symbols.
- **Don’t** wrap Share in a new `shell-panel*` / `card-glass` (**C653**).
- **Don’t** add Share to Pool / Limits in #665. Swap header Share is **#713**. Pair `/trade` / `/charts` mounts are optional; `/portfolio` must emit `/trader/{wallet}`, never `/portfolio`.

## Surfaces

| Route | When | Payload |
|-------|------|---------|
| `/trader/:address` | Valid terra address (incl. 404 / outage) | `/trader/{addr}` |
| `/trader` or invalid segment | Hidden | — |
| `/portfolio` (optional, shipped) | Connected + valid wallet | `/trader/{wallet}` |
| `/trade/:pairAddr` / `/charts/:pairAddr` | Optional follow-up; helper already allows `trade` / `charts` kinds | Canonical pair path after existing validation |
| `/` Swap | Required in [#713](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/713) | `{origin}/?from=&to=` via `buildCanonicalSwapShareUrl` |

## Verification

```bash
make verify-issue-665
```

Vitest covers URL build, abort vs fallback, TraderPage presence, AddressRow still copies bech32. Playwright `e2e/trader-page.spec.ts` asserts `trader-share-link` (5 workers, no `e2e-tx`). `python3 scripts/check_chrome_nesting.py` stays green.

## Cross-links

- Clipboard primitive: [`AGENTS_FRONTEND_COPY_BUTTON.md`](./AGENTS_FRONTEND_COPY_BUTTON.md) ([#183](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/183))
- Address row: [`AGENTS_FRONTEND_ADDRESS_ROW.md`](./AGENTS_FRONTEND_ADDRESS_ROW.md) ([#188](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/188))
- Retail copy: [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) ([#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/489))
- Open Graph (do not regress): [`AGENTS_FRONTEND_OPENGRAPH.md`](./AGENTS_FRONTEND_OPENGRAPH.md) ([#578](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/578))
- Chrome nesting: [`AGENTS_FRONTEND_CHROME_NESTING.md`](./AGENTS_FRONTEND_CHROME_NESTING.md) ([#653](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/653))
- Focus rings: [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md) ([#144](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/144))
- Portfolio: [`AGENTS_FRONTEND_PORTFOLIO.md`](./AGENTS_FRONTEND_PORTFOLIO.md)
- Post-merge leftover: [#673](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/673) / `make verify-issue-673` / [`AGENTS_POST_MERGE_OPS_673.md`](./AGENTS_POST_MERGE_OPS_673.md)
- Swap Share / URL sync: [`AGENTS_FRONTEND_SWAP_URL_SYNC.md`](./AGENTS_FRONTEND_SWAP_URL_SYNC.md) ([#713](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/713))
