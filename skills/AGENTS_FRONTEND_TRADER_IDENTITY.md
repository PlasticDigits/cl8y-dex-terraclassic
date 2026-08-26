# Agent playbook: trader identity (4/6 + blockie PFP)

Audience: third-party agents adding trader-as-person chrome on Charts leaderboard, `/trader/:addr`, `/portfolio`, or any new “top traders” row.

**Issue:** [GitLab **#656**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/656)  
**Invariants:** [`docs/frontend.md` § Trader identity](../docs/frontend.md#trader-identity) (**T-ID-1–T-ID-10**)  
**Related:** [#188](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/188) `AddressRow`, [#541](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/541) token identity (tokens ≠ traders), [#553](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/553) leaderboard Volume USD, [#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378) logo allowlist, [#430](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/430) explorer hrefs, [#186](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/186) wallet chip, [#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/489) retail copy, [#653](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/653) one chrome layer.

## Problem class

Retail trader surfaces used to print a long `terra1…` bech32 as the only identity (leaderboard **10/6**, profile **12/6**, no PFP). Truncation and avatar are **one** treatment: a **4/6** chip plus a **deterministic blockie** from existing `react-blockies`. Do not split them.

## Do / don’t

- **Do** use [`TraderBlockie`](../frontend-dapp/src/components/trader/TraderBlockie.tsx) + [`TraderIdentity`](../frontend-dapp/src/components/trader/TraderIdentity.tsx). Shorten with [`shortenTraderAddress`](../frontend-dapp/src/utils/tokenDisplay.ts) / `TRADER_ADDR_START_CHARS` / `TRADER_ADDR_END_CHARS`.
- **Do** seed blockies with **lowercase** bech32. Paint only when `isValidTerraAddress` passes.
- **Do** keep `Link` `to`, copy payload, explorer `href`, React `key`, and `/trader/:address` as the **full** validated bech32.
- **Do** keep profile copy + explorer via `AddressRow` at 4/6. Leaderboard is link-only (click → profile).
- **Don’t** reuse [`TokenLogo`](../frontend-dapp/src/components/ui/TokenLogo.tsx) as a trader PFP (allowlisted bitmaps vs generated-only).
- **Don’t** load `logo_url` / ENS / Gravatar / NFT avatars. Do not extend [`tokenLogoAllowlist.ts`](../frontend-dapp/src/utils/tokenLogoAllowlist.ts).
- **Don’t** add a second identicon library. Do not change `shortenAddress` defaults (8/6).
- **Don’t** retune the connected-wallet chip (#186 6/6 / 4/4). Tape and order-book owner tooltips stay out of scope.
- **Don’t** wrap leaderboard rows or the profile blockie in `card-glass` (**C653-1**).
- **Don’t** put the 4/6 string into share / QR / clipboard payloads.

## Invariants

| ID | Meaning |
|----|---------|
| **T-ID-1** | Leaderboard visible label is `shortenAddress(addr, 4, 6)` — not 10/6, not the full bech32. |
| **T-ID-2** | Valid rows show a circular blockie left of the short label. Same address → same seed; different addresses → different seeds. |
| **T-ID-3** | Leaderboard `href` is `/trader/{full bech32}` (entire identity is one `Link`). |
| **T-ID-4** | `/trader/:addr` and `/portfolio` headers show a larger blockie from the same primitive beside `AddressRow`. |
| **T-ID-5** | Profile `AddressRow` is 4/6. Copy is the **full** address. Explorer only via `getExplorerAddressUrl`. |
| **T-ID-6** | Invalid / non-`terra1` strings: no blockie, no `/trader/` or explorer link. |
| **T-ID-7** | No new npm identicon. No trader `logo_url`. No `GET /api/v1/traders/*` JSON change. |
| **T-ID-8** | Token logos (#541), wallet chip (#186), tape, order-book owner tooltip unchanged. |
| **T-ID-9** | One line at 375px (blockie + 4/6). No nested chrome card. |
| **T-ID-10** | One shared primitive — do not hand-roll a second blockie in Charts vs profile. |

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/utils/tokenDisplay.ts` | `shortenTraderAddress`, `TRADER_ADDR_*` |
| `frontend-dapp/src/components/trader/TraderBlockie.tsx` | `react-blockies`, circular clip, `aria-hidden` |
| `frontend-dapp/src/components/trader/TraderIdentity.tsx` | Compact chip + optional `/trader/{full}` `Link` |
| `frontend-dapp/src/pages/ChartsPage.tsx` | Leaderboard Trader cell |
| `frontend-dapp/src/components/trader/TraderSummaryStats.tsx` | Profile + portfolio header |

## Regression

```bash
make verify-issue-656
```

Vitest: `tokenDisplay.test.ts` (`shortenTraderAddress`), `TraderBlockie.test.tsx`, `TraderIdentity.test.tsx`, `ChartsPage.test.tsx` (#656), `TraderSummaryStats.test.tsx` (#656), `AddressRow.test.tsx`. No LocalTerra / Postgres.

## Related

- [`AGENTS_FRONTEND_ADDRESS_ROW.md`](./AGENTS_FRONTEND_ADDRESS_ROW.md) — trader header 4/6 + blockie; defaults stay 8/6
- [`AGENTS_FRONTEND_TOKEN_IDENTITY.md`](./AGENTS_FRONTEND_TOKEN_IDENTITY.md) — tokens ≠ traders
- [`AGENTS_FRONTEND_TRADER_VOLUME_USD.md`](./AGENTS_FRONTEND_TRADER_VOLUME_USD.md) — Volume USD column stays #553
- [`AGENTS_FRONTEND_WALLET_CHIP.md`](./AGENTS_FRONTEND_WALLET_CHIP.md) — chip shorten is #186, out of scope
- [`AGENTS_FRONTEND_TRUST_BOUNDARIES.md`](./AGENTS_FRONTEND_TRUST_BOUNDARIES.md) — no remote trader avatars
- [`AGENTS_FRONTEND_CHROME_NESTING.md`](./AGENTS_FRONTEND_CHROME_NESTING.md) — no `card-glass` on leaderboard rows
- [`AGENTS_FRONTEND_COPY_BUTTON.md`](./AGENTS_FRONTEND_COPY_BUTTON.md) — copy full bech32 only
- [`AGENTS_FRONTEND_TERRA_EXPLORER.md`](./AGENTS_FRONTEND_TERRA_EXPLORER.md) — explorer href helper only
- [`AGENTS_FRONTEND_PORTFOLIO.md`](./AGENTS_FRONTEND_PORTFOLIO.md) — shared `TraderSummaryStats`
