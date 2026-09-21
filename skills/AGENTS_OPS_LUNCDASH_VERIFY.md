# Agent playbook: leftover Lunc Dash WalletConnect verify (#1279)

Use when an agent is assigned leftover **[#1279](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1279)** (*incident(frontend): verify Lunc Dash wallet path on DEX*), or when changing Connect Wallet **LuncDash**, `buildLuncDashDeepLink`, Lunc Dash UA / terms hint, or Station/LuncDash atomic WalletConnect post.

This is **ops verification leftover** of an already-shipped path. It is **not** a product implement ticket and **not** a reopen of closed connect bugs.

**Issue:** [#1279](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1279)  
**Verify (implement pre-check):** `make verify-issue-1279`  
**QA invariant:** [`docs/qa-invariants.md`](../docs/qa-invariants.md) **Q20** (**L1279-1–L1279-8**)  
**Shipped children (closed unless a merged AC is actually broken):** [#519](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/519) same-device pairing, [#554](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/554) Android Chrome pairing foreground, [#566](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/566) WC matrix (Lunc Dash was already WC), [#658](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/658) non-Keplr terms hint, [#490](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/490) logos.

Product pairing playbook: [`AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md`](./AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md) (**WC-M1–WC-M12**). Legal hint: [`AGENTS_FRONTEND_CLICKWRAP.md`](./AGENTS_FRONTEND_CLICKWRAP.md) (**WC-M12** / **L658**). Atomic WC post: [`AGENTS_FRONTEND_STATION_SIGNING.md`](./AGENTS_FRONTEND_STATION_SIGNING.md) / [ADR 0004](../docs/adr/0004-terraclassic-retail-gas-census.md) G3.

## Invariants (L1279-1–L1279-8)

| ID | Rule |
|----|------|
| **L1279-1** | Product path already shipped. Do **not** invent a second Lunc Dash URI scheme. Do **not** add Leap. Do **not** implement Legal portal ADR-036 in this repo. Do **not** treat wallet-app LCD-sim gas rewrite as a DEX broadcast bug (ADR 0004 Stay). Do **not** remove in-app browser as a documented alternate path (**WC-M7**). |
| **L1279-2** | `make verify-issue-1279` is **pre-check only** (Vitest + docs + child 519/554/658 unless `VERIFY1279_SKIP_CHILDREN=1`). Green make does **not** close leftover. |
| **L1279-3** | Connect always emits `name: 'LuncDash'`, `WalletName.LUNCDASH`, `WalletType.WALLETCONNECT` on desktop **and** mobile. No extension install row. Leap stays absent. |
| **L1279-4** | Keep `buildLuncDashDeepLink` (`luncdash://wallet_connect?` + encoded `payload=`). Allowlist `luncdash:`. Prefer this URI when `details.isStation && details.isLuncDash`. Mobile: **Open Lunc Dash** / **Open wallet** (`wc:`) / **Copy pairing link** / **Cancel**. Desktop QR unchanged (**WC-M2**). |
| **L1279-5** | Station / LuncDash WalletConnect stay on atomic `broadcastTx` (`isAtomicWalletConnectPost`). Do not LCD-sim in the dApp; do not re-enable mainnet SEC-E08. |
| **L1279-6** | Unsigned WC terms hint names **Lunc Dash** (or the DEX wallet list) — not Keplr-only (**WC-M12** / **L658**). UA `/LuncDash\|LUNCDash\|LUNC Dash/i` → `Lunc Dash`. Hide the hint when a keplr-like injector is present. |
| **L1279-7** | Leftover-complete is operator [`QA_TEMPLATE.md`](../QA_TEMPLATE.md) **1.5** on a real Lunc Dash install, assigned as **cl8y-pm inbox cards** (phone or desktop; at most five taps each). `VERIFY1279_IID=1279` / `VERIFY1279_LEFTOVER_COMPLETE=1` **must FAIL** (do not close leftover on make). There is **no** ops-bot channel. Do not dump 1.5.1–1.5.11 as one card. Do not paste tokens on the issue or card. |
| **L1279-8** | This playbook + **Q20** + WC-M / L658 skills stay crosslinked. Reopen the matching **closed** child only with device AC evidence. Listing CW20s *inside* Lunc Dash’s own registry is [#1260](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1260), not this ticket. |

## Operator leftover (do not implement here)

File **cl8y-pm inbox cards** for the human device checks (one founder; assigned load must be visible). Split [`QA_TEMPLATE.md`](../QA_TEMPLATE.md) **1.5** into cards of at most five numbered taps. Typical split:

1. Pair: Connect → LuncDash → QR on a second device **or** same-device **Open Lunc Dash** / **Copy pairing link** → `terra1…` in the header.
2. Sign: Swap approve succeeds; reject shows a rejection (not a hang). Disconnect / tab reopen if it still fits the same card’s five taps.

Put https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1279 in the card body. Close #1279 after the cards are done (or reopen the matching closed defect if an AC failed).

## Do / don’t

- **Do** run `make verify-issue-1279` from a git worktree after pulling `main`.
- **Do** link `frontend-dapp/node_modules` from the primary checkout in a git worktree. Do **not** `npm install` over a worktree `node_modules` symlink.
- **Do** keep `useWallet` constants mock as `importOriginal` + `DEV_MODE: true` so child #554 can import `tokenRegistry` (`LUNC_C_TOKEN_ADDRESS`).
- **Do** POST cl8y-pm inbox cards for leftover device work the agent cannot run.
- **Don’t** close leftover on unit tests or this make target.
- **Don’t** invent an ops-bot channel or file an 11-step inbox novel.
- **Don’t** reopen #519 / #554 / #566 / #658 / #490 without device evidence of a closed AC.
- **Don’t** add Leap, a second URI scheme, Legal ADR-036, or a DEX LCD-sim of Station/LuncDash WC post.

## Regression

```bash
make verify-issue-1279
# docs + Lunc Dash Vitest only (skip child 519/554/658):
VERIFY1279_SKIP_CHILDREN=1 make verify-issue-1279
# leftover-complete must FAIL (inbox cards close the issue, not make):
VERIFY1279_IID=1279 make verify-issue-1279          # expected non-zero
VERIFY1279_LEFTOVER_COMPLETE=1 make verify-issue-1279
```

## Cross-links

- Pairing UX: [`AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md`](./AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md) (**WC-M1–WC-M12**)
- Connect list / logos: [`AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md`](./AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md) (**D1–D9**)
- Terms hint: [`AGENTS_FRONTEND_CLICKWRAP.md`](./AGENTS_FRONTEND_CLICKWRAP.md) (**L658-1–L658-8**)
- Atomic WC post / gas residual: [`AGENTS_FRONTEND_STATION_SIGNING.md`](./AGENTS_FRONTEND_STATION_SIGNING.md), [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md), [ADR 0004](../docs/adr/0004-terraclassic-retail-gas-census.md)
- Retail docs: [`docs/frontend.md`](../docs/frontend.md#walletconnect-same-device-mobile)
- QA matrix: [`QA_TEMPLATE.md`](../QA_TEMPLATE.md) § 1.5 · [`docs/qa-onboarding.md`](../docs/qa-onboarding.md) § Wallet Matrix
- Verify: [`scripts/qa/verify-issue-1279.sh`](../scripts/qa/verify-issue-1279.sh)
