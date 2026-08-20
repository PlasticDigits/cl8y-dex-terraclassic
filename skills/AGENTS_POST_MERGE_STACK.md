# Agent playbook: post-merge Coolify + indexer stack (!368–!377 / GitLab #573)

Audience: third-party agents verifying a **stacked frontend/indexer merge** after GitLab CI was skipped (`ci_quota_exceeded`), or shipping Coolify + indexer together so logo, WalletConnect, Ledger copy, gem hide, hub P&amp;L, and tape decimals land as one production cut.

**Issue:** [GitLab **#573**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/573)  
**Invariants:** [`docs/qa-invariants.md`](../docs/qa-invariants.md) **Q6** (**M573-1–M573-8**)  
**Verify:** `make verify-issue-573`

## Stacked MRs on `main`

| MR | Issue | What must ship together |
|----|-------|-------------------------|
| !368 | (no issue) | Simplified C+8 favicon — never downscale `/logo.png` |
| !369 | [#566](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/566) | Station + Cosmostation WalletConnect |
| !370 + !371 | [#564](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/564) / [#565](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/565) | Charts 24h Stats USD + TWAP/histogram human scale |
| !372 | [#560](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/560) | Hub-priced portfolio/trader P&amp;L |
| !373 | [#567](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/567) | Keplr + Ledger amino / stall UX |
| !374 | [#563](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/563) | Trade ticket **Buy {base}** + Buy/Sell colors |
| !375 | [#557](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/557) | Tape Amount in/out/Price human scale |
| !376 | [#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562) | Hide soft-launch gems on production |
| !377 | [#561](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/561) | Flatten `/trade` desktop layout |

## Invariants (M573-1–M573-8)

| ID | Rule |
|----|------|
| **M573-1** | Local regression is `make verify-issue-573`, which runs child verifies **557, 560, 561, 562, 563, 564, 565, 566, 567**. A child FAIL fails the stack. SKIP (no LocalTerra) is allowed only for optional Playwright rungs, not for unit/docs. |
| **M573-2** | Coolify frontend rebuild (`npm ci` / image rebuild) so !368–!377 ship together. Production bake: `VITE_NETWORK=mainnet`, **`VITE_SHOW_TEST_TOKENS` unset**, **`VITE_FAUCET_ADDRESS` unset** (**P562-1** / **F11**). |
| **M573-3** | Indexer restart after additive trade/fill decimals (#557) and optional position decimals (#560). Confirm #553 / #556 migrations if they are not already live. JSON amount columns stay **raw**. |
| **M573-4** | LocalTerra Swap pay list **still includes EMBER** (P1). Gems are omitted only when `VITE_NETWORK=mainnet` and the show-test override is unset (**P562-3**). Spec: `frontend-dapp/e2e/retail-test-tokens-562.spec.ts`. |
| **M573-5** | Favicons use the simplified C+8 mark (`favicon-16.png` / `favicon-32.png` / `logo-simplified-variant.png`). Do **not** wire `/logo.png` (full character scene) as a tab icon. |
| **M573-6** | Hardware / device AC stays operator-owned: Android Chrome Station + Cosmostation WC (**#566**), columbus-5 Keplr + Ledger Nano (**#567**). Local unit+docs passing does **not** close those rungs. |
| **M573-7** | Production smoke after Coolify: Charts UST1/cUSTC tape vs LCD raw ÷ 10^6 (**#557**); Charts UST1/USTR no compact `T`; Vol (USD) ≈ stats `volume_usd` (**#564** / **#565**); desktop 1280/1440 `/trade` + ticket heading **Buy cLUNC** at ~320px (**#561** / **#563**); trader header USD ≈ hub UST1 × human P&amp;L (**#560**); production has no gemstone tickers + faucet **Pause** (**#562** F9). |
| **M573-8** | This playbook + **Q6** + child skills stay crosslinked. Do not wait for GitHub Actions; GitLab CI may be quota-blocked — local `make verify-issue-*` is the gate. |

## Do / don’t

- **Do** run `make verify-issue-573` from a git worktree after pulling `main`.
- **Do** provision LocalTerra (`make setup-cloud-localterra`) when child Playwright SKIPs and the issue asks for P1.
- **Do** keep indexer `target/` host-owned (`make test-indexer-target-ownership`) — cargo in a root Docker bind-mount is **not** a product defect (#557/#560).
- **Don’t** treat local verify as Coolify deploy. Rebuild the production frontend image and restart the indexer separately.
- **Don’t** hide gems on LocalTerra to “match production.” `VITE_NETWORK=local` must still list EMBER (**M573-4**).
- **Don’t** close #573 while **M573-6** / **M573-7** remain unexecuted on columbus-5.

## Regression

```bash
make verify-issue-573
# docs/logo/crosslinks only:
VERIFY573_SKIP_CHILDREN=1 make verify-issue-573
```

Child playbooks: [`AGENTS_FRONTEND_TAPE_AMOUNTS.md`](./AGENTS_FRONTEND_TAPE_AMOUNTS.md), [`AGENTS_FRONTEND_HUB_PNL.md`](./AGENTS_FRONTEND_HUB_PNL.md), [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md), [`AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](./AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md), [`AGENTS_FRONTEND_TRADE_TICKET_HEADING.md`](./AGENTS_FRONTEND_TRADE_TICKET_HEADING.md), [`AGENTS_FRONTEND_CHARTS_PAIR_STATS.md`](./AGENTS_FRONTEND_CHARTS_PAIR_STATS.md), [`AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md`](./AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md), [`AGENTS_FRONTEND_KEPLR_LEDGER.md`](./AGENTS_FRONTEND_KEPLR_LEDGER.md).

Coolify env: [`docs/runbooks/mainnet-soft-launch.md`](../docs/runbooks/mainnet-soft-launch.md).
