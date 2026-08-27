# Agent playbook: trailing 24h / 7d / 30d volume copy (GitLab #576)

Audience: third-party agents changing Charts pair 24h Stats, Protocol global volume, Pool **Vol** header, or `StatBox` `title` / `aria-label`.

**Issue:** [GitLab **#576**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/576)  
**Invariants:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) row **Trailing window copy (#576)**; [`docs/frontend.md`](../docs/frontend.md) § [Charts pair-scoped](../docs/frontend.md#charts-pair-scoped), [Charts pair 24h stats](../docs/frontend.md#charts-pair-stats), [Protocol](../docs/frontend.md#protocol-page)  
**Glossary:** [`docs/design-system.md`](../docs/design-system.md) § Terminology — **24h volume**  
**Copy skill:** [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) (#489)

## Problem class

Retail reads **24h Volume (USD)** as a calendar-day counter that should hit `$0` at midnight. The indexer figure is `Utc::now() − 24h` over `swap_events`. On a live DEX the number stays above `$0` and can trend up for days. That is intended. This issue is **copy + progressive disclosure**, not a window change.

Related (do **not** treat as done here): [#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548) USD-only overview; [#550](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550) Protocol 24h/7d/30d math; [#565](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/565) pair Vol (USD); [#577](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/577) rollup decay; [#586](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/586) Protocol fee titles (`TRAILING_*_FEES_*`) reuse the same trailing-window wording.

## Invariants (W1–W5)

| ID | Rule |
|----|------|
| **W1** | Charts pair **Last 24h Vol (USD)** / **Last 24h Trades** and Protocol **24h** volume disclose a trailing window (visible **Last 24h** + `title` / `aria-label`). No hover required. `/charts` has no DEX-census overview strip ([#666](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/666)). |
| **W2** | Pair **Vol (USD)** and Protocol **24h / 7d / 30d** volume **and fees** use the same trailing wording (7d/30d = last N days). |
| **W3** | Visible labels ≤ ~5 words. No always-on educational banner on `/charts` or `/protocol`. No raw `total_volume_24h`. |
| **W4** | Idle DEX still `$0`; unpriced + trades still `—`; USD compact format unchanged (**C1–C3**). `$0` means idle window, not daily close. |
| **W5** | Copy is static constants in [`trailingWindowCopy.ts`](../frontend-dapp/src/utils/trailingWindowCopy.ts). Never interpolate overview JSON into `title`. No `VITE_INDEXER_URL` / hostnames. No “guaranteed USD” / settlement (**X5**). |

## Do / don’t

- **Do** keep indexer cutoff `now − 24h` (and 7d / 30d). No UTC-day reset, no “resets at 00:00”.
- **Do** put `title` on the StatBox **label** `<p>` (mobile long-press) and `aria-label` on the value (`title + value`).
- **Do** reuse [`TRAILING_24H_VOLUME_TITLE`](../frontend-dapp/src/utils/trailingWindowCopy.ts) on Charts pair Vol and Protocol 24h.
- **Don’t** switch to calendar-day volume to make the number hit `$0` (desyncs CG/CMC, pair stats, and every cutoff test).
- **Don’t** add a help `?` overlay or lecture paragraph.
- **Don’t** change `GET /api/v1/overview` field semantics in this issue.
- **Don’t** retitle the Protocol UTC grain chart (Hourly / Daily / Monthly, [#668](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/668) / [#677](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/677)) as Last 24h/7d/30d — that chart is calendar buckets, not the trailing tiles. X-axis density is **P668-9**.

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/utils/trailingWindowCopy.ts` | Shared labels + titles |
| `frontend-dapp/src/components/ui/StatBox.tsx` | `title` on card + label; value `aria-label` |
| `frontend-dapp/src/pages/ChartsPage.tsx` | Pair Vol + SORT label (no DEX census strip) |
| `frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx` | 24h / 7d / 30d + trades |
| `frontend-dapp/src/components/pool/PoolPairsTable.tsx` | Optional Vol header `title`; sort key stays `volume_24h` |

## Regression

```bash
make verify-issue-576
```

## Related

- [`AGENTS_FRONTEND_CHARTS_OVERVIEW.md`](./AGENTS_FRONTEND_CHARTS_OVERVIEW.md) — USD-only overview API / Protocol (**C1–C9**, #548)
- [`AGENTS_FRONTEND_CHARTS_PAIR_SCOPED.md`](./AGENTS_FRONTEND_CHARTS_PAIR_SCOPED.md) — Charts pair-only layout (#666)
- [`AGENTS_FRONTEND_CHARTS_PAIR_STATS.md`](./AGENTS_FRONTEND_CHARTS_PAIR_STATS.md) — pair Vol (USD) (#565)
- [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) — Protocol USD census (#550)
- [`AGENTS_FRONTEND_POOL_TABLE.md`](./AGENTS_FRONTEND_POOL_TABLE.md) — `/pool` Vol column (#547)
- [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) — #489
- [`AGENTS_INDEXER_VOLUME_PAGINATION.md`](./AGENTS_INDEXER_VOLUME_PAGINATION.md) — rollup + 60s cache
