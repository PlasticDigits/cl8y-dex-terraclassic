# Agent playbook: one chrome layer / anti-nesting

Audience: third-party agents adding page sections, `StatBox` grids, or `card-glass` inside `shell-panel*`.

**Issue:** [GitLab **#653**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/653)  
**Invariants:** [`docs/frontend.md` § One chrome layer](../docs/frontend.md#one-chrome-layer) (**C653-1–C653-8**)  
**Spec:** [`docs/design-system.md`](../docs/design-system.md) principle **One chrome layer per region**  
**Related:** [#561](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/561) Trade application (**L561-1–L561-2**), [#652](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/652) Protocol inline Δ% (do not restyle Global stats/fees beyond `flat` here), [#488](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/488) tokens, [#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/489) copy density.

## Problem class

The design-system sentence used to bless nested `card-glass` inside a page `shell-panel` for “distinct inner blocks.” Agents then dropped default `StatBox` (`card-glass !p-3`) into section panels. `/protocol` Global stats, Charts overview, and Trader summary became a card of cards. Metric tiles are **content**, not a second chrome region.

## Do / don’t

- **Do** stack: page `--bg-*` → **one** `shell-panel*` per region → typography / CSS grid / hairline dividers.
- **Do** use `StatBox variant="flat"` (or `.stat-flat`) for label+value census tiles inside a panel. Default `card` stays for isolated tiles that are the only surface.
- **Do** keep Swap Pay/Receive `card-glass` (`swap-io-card-*`) inside the swap panel — that is the **canonical** interactive exception.
- **Do** keep Trade book / chart / ticket / tape as **sibling** `shell-panel*` / `card-glass` cells. Do **not** wrap `PriceChart` (`shell-panel-strong`) in `card-glass` (**L561-1**).
- **Do** allow **one** inner `card-glass` well for a single table or chart (Protocol oracle history, Protocol hooks table).
- **Don’t** wrap a grid of 4–11 `card-glass` / default `StatBox` chips in a section panel.
- **Don’t** wrap every `<tr>` in `card-glass`.
- **Don’t** silently flip `StatBox` default to `flat` — call sites inside a panel pass `variant="flat"`.
- **Don’t** invent a new color system or gold fills. Tokens stay #488.
- **Don’t** change overview JSON / indexer semantics while flattening chrome.
- **Don’t** implement [#652](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/652) inline Δ% / daily volume chart in this pass. Consume `flat` there when that issue lands.

## Invariants

| ID | Meaning |
|----|---------|
| **C653-1** | One chrome layer per visual region. Forbidden: `shell-panel*` wrapping `shell-panel*`, or wrapping a **grid of** `card-glass` / default `StatBox`. |
| **C653-2** | Allowlist is short: Swap IO cards; a **single** table/chart well; Trade **sibling** panels. Snapshot: [`scripts/chrome_nesting_allowlist.txt`](../scripts/chrome_nesting_allowlist.txt). |
| **C653-3** | Metric tiles use `StatBox variant="flat"` / `.stat-flat`: no second radius, border, or blur. Default `StatBox` stays `card`. |
| **C653-4** | Charts pair 24h + TWAP grids, Trader summary (including P&L chips), Protocol Global stats / fees / oracle stat chips: flat. Hub prices were already typographic `dl`. Charts has **no** DEX-census overview strip ([#666](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/666)). |
| **C653-5** | Swap Pay/Receive stay `card-glass`. Trade chart stays a single `shell-panel-strong` (**L561-1**). No `PanelResizeHandle`. |
| **C653-6** | `title` / `aria-label` on StatBox survive a variant change. Testids unchanged. No indexer JSON change. |
| **C653-7** | Mechanical guard: `python3 scripts/check_chrome_nesting.py` (static regex, no `eval`). New same-file `shell-panel`+`card-glass` hits fail unless allowlisted. |
| **C653-8** | Light + dark; 375px / 1280px: numbers stay `--ink` on `--panel-bg` (not leftover `--ink-subtle` on a missing `--card-bg` ring). No lecture banner. |

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/components/ui/StatBox.tsx` | `variant` `card` \| `flat` |
| `frontend-dapp/src/index.css` | `.stat-flat` primitive |
| `frontend-dapp/src/pages/ChartsPage.tsx` | Overview + pair 24h + TWAP flat |
| `frontend-dapp/src/components/trader/TraderSummaryStats.tsx` | Profile + P&L flat |
| `frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx` | Flat tiles (inline Δ% is #652) |
| `frontend-dapp/src/components/protocol/ProtocolFeeStats.tsx` | Flat tiles (inline Δ% is #652) |
| `frontend-dapp/src/components/protocol/ProtocolOracleCard.tsx` | Flat StatBoxes; one table well |
| `scripts/check_chrome_nesting.py` | CI / verify guard |
| `scripts/chrome_nesting_allowlist.txt` | Remaining interactive / well nests |

## Regression

```bash
make verify-issue-653
```

Vitest: `StatBox.test.tsx`, `ChartsPage.test.tsx` overview chrome, `TraderSummaryStats.test.tsx`. Docs/skills must not restore the blanket “nested card-glass is OK for distinct inner blocks.”

Trade chrome still: `make verify-issue-561`. Charts numbers: `make verify-issue-548`. Protocol numbers: `make verify-issue-550`. Sibling layout (inline Δ%): [#652](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/652).

## Related

- [`AGENTS_FRONTEND_DESIGN_SYSTEM.md`](./AGENTS_FRONTEND_DESIGN_SYSTEM.md) — tokens / primitives (#488)
- [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md) — `/trade` sibling panels (**L561**)
- [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) — nested chrome is visual noise (#489)
- [`AGENTS_FRONTEND_CHARTS_OVERVIEW.md`](./AGENTS_FRONTEND_CHARTS_OVERVIEW.md) — overview USD (#548)
- [`AGENTS_FRONTEND_CHARTS_PAIR_STATS.md`](./AGENTS_FRONTEND_CHARTS_PAIR_STATS.md) — pair 24h / TWAP
- [`AGENTS_FRONTEND_TRADER_VOLUME_USD.md`](./AGENTS_FRONTEND_TRADER_VOLUME_USD.md) — trader volume USD
- [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) — `/protocol` census; #652 owns inline Δ%
