# Agent playbook: compact token identity (Pool / Trade / Charts)

Audience: third-party agents adding explorer, copy, or “token page” chrome on `/pool`, `/trade`, or `/charts`.

**Issue:** [GitLab **#541**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/541)  
**Invariants:** [`docs/frontend.md` § Token identity](../docs/frontend.md#token-identity) (**T541-1–T541-8**)  
**Related:** [#188](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/188) `AddressRow`, [#184](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/184) / [#478](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/478) explorer URLs, [#430](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/430) SEC-E10, [#183](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/183) `CopyButton`, [#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/489) anti-cognitive-overload, [#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378) trust boundaries, [#524](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/524) display invert.

## Problem class

Pair **symbols are not identity**. Terra Classic has look-alike CW20s. Pool / Trade / Charts must expose a **quiet** copy + explorer control for both legs and the pair contract — without address essays, `/token` routes, or Protocol factory/router clones.

## Do / don’t

- **Do** use [`tokenIdentityTarget`](../frontend-dapp/src/utils/tokenIdentity.ts) + [`TokenIdentity`](../frontend-dapp/src/components/ui/TokenIdentity.tsx) / [`PairTokenLinks`](../frontend-dapp/src/components/ui/PairTokenLinks.tsx). Pair contract stays [`AddressRow`](../frontend-dapp/src/components/ui/AddressRow.tsx).
- **Do** send every explorer `href` through `getExplorerAddressUrl`. Invalid / `javascript:` / HTML / non-bech32 → `null` → omit the anchor.
- **Do** copy the **contract or denom**, never the display symbol.
- **Do** keep explorer as a **sibling** of `TokenDisplay`. Do not wrap the symbol / pair-select label in `<a>`.
- **Do** key the row on the pair address; hide it on #176 / #175 / empty “No pairs yet”.
- **Do** keep #524 invert as **label/order only**. `token-identity-base` = factory `asset_0`, `token-identity-quote` = factory `asset_1`.
- **Don’t** invent a Finder URL for `uluna` / `uusd` / any `native_token` denom (copy-only).
- **Don’t** put identity icons inside `PairSearchSelect` / `TokenSearchSelect` option rows.
- **Don’t** add `/token/:id`, CoinGecko, CMC, Twitter, or indexer `website` / `logo_url` hosts.
- **Don’t** clone factory/router `AddressRow` onto these pages (`/protocol` only).
- **Don’t** change Swap confirm, wallet menu, trader header, or LP withdraw `pool-lp-token-address-row` except shared icon/CSS. Trader-as-person chrome is [#656](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/656) ([`AGENTS_FRONTEND_TRADER_IDENTITY.md`](./AGENTS_FRONTEND_TRADER_IDENTITY.md)) — not `TokenLogo`.
- **Don’t** ship always-on address essays or “use Wrap / UST1 / Swap” banners ([#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/489) invariant **9**).

## Invariants

| ID | Meaning |
|----|---------|
| **T541-1** | Pool, Trade, and Charts each expose copy + (when applicable) explorer for **both pair legs** and the **pair contract** without opening a wallet menu. |
| **T541-2** | CW20 / pair `href` is only `getExplorerAddressUrl`; native is copy-only. |
| **T541-3** | Symbol / logo never wrap an `<a>`; explorer is a sibling control. |
| **T541-4** | Copy payload is contract or denom, never the display symbol. |
| **T541-5** | Display invert does not swap explorer/copy targets. |
| **T541-6** | Invalid / missing pair: no identity links. |
| **T541-7** | No new outbound hosts, no `/token` route, no picker-option icons, no Protocol factory/router clone. |
| **T541-8** | `#489` / `#378`: no always-on address essay or cross-nav banner. |

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/utils/tokenIdentity.ts` | `tokenIdentityTarget`, invert-stable `pairIdentityTargets` |
| `frontend-dapp/src/components/ui/TokenIdentity.tsx` | Logo + symbol + copy + optional explorer |
| `frontend-dapp/src/components/ui/PairTokenLinks.tsx` | One row: legs + pair `AddressRow` |
| `frontend-dapp/src/pages/PoolPage.tsx` / `PoolPairsTable.tsx` | Identity on each table row; LP `AddressRow` on Manage expand ([#547](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547)) |
| `frontend-dapp/src/pages/TradePage.tsx` | Under `trade-pair-select-panel`, outside the combobox |
| `frontend-dapp/src/pages/ChartsPage.tsx` | Under the pair `MenuSelect` |

## Regression

```bash
make verify-issue-541
```

Vitest: `tokenIdentity.test.ts`, `TokenIdentity.test.tsx`, `TokenIdentity.explorerSafety.test.tsx`, `PairTokenLinks.test.tsx`, scoped Pool / Trade / Charts `#541` describes. Playwright smoke: `e2e/token-identity-541.spec.ts` (5 workers, no e2e-tx).

## Related

- [`AGENTS_FRONTEND_ADDRESS_ROW.md`](./AGENTS_FRONTEND_ADDRESS_ROW.md) — pair chips leftover closed by #541
- [`AGENTS_FRONTEND_PROTOCOL_HUB.md`](./AGENTS_FRONTEND_PROTOCOL_HUB.md) — Protocol hub wrap identity is #570, not a Pool/Trade/Charts clone
- [`AGENTS_FRONTEND_TERRA_EXPLORER.md`](./AGENTS_FRONTEND_TERRA_EXPLORER.md) — href helper only
- [`AGENTS_FRONTEND_COPY_BUTTON.md`](./AGENTS_FRONTEND_COPY_BUTTON.md)
- [`AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](./AGENTS_FRONTEND_TRADE_PAIR_INVERT.md) — invert must not swap identity payloads
- [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) — no address essays
- [`AGENTS_FRONTEND_TRUST_BOUNDARIES.md`](./AGENTS_FRONTEND_TRUST_BOUNDARIES.md) — no Protocol factory/router clone
- [`AGENTS_FRONTEND_NATIVE_TICKERS.md`](./AGENTS_FRONTEND_NATIVE_TICKERS.md) — visible **LUNC** / **USTC**; copy payload stays `uluna` / `uusd` (**N630-7**, [#630](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/630))
- [`AGENTS_FRONTEND_TRADER_IDENTITY.md`](./AGENTS_FRONTEND_TRADER_IDENTITY.md) — traders ≠ tokens; do not reuse `TokenLogo` as a wallet PFP ([#656](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/656))
