# Agent playbook: retail error humanization (frontend)

Use when surfacing **wallet**, **fetch/indexer**, or **mutation** failures in the React dApp, or when reviewing UX that currently prints **`error.message`** verbatim ([GitLab **#145**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/145)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § User-facing errors](../docs/frontend.md#user-facing-errors-humanization) | Invariant table, component funnel |
| [`frontend-dapp/src/utils/humanizeUserFacingError.ts`](../frontend-dapp/src/utils/humanizeUserFacingError.ts) | **`humanizeUserFacingError`**, **`humanizeUserFacingErrorFromUnknown`**, **`getErrorMessage`** |
| [`frontend-dapp/src/utils/humanizeOffChainError.ts`](../frontend-dapp/src/utils/humanizeOffChainError.ts) | Wallet / transport try-match helpers + **`sanitizeOpaqueErrorMessage`** |
| [`frontend-dapp/src/utils/humanizeTerraTxError.ts`](../frontend-dapp/src/utils/humanizeTerraTxError.ts) | On-chain / LCD patterns ([GitLab **#134**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/134)) |
| [`frontend-dapp/src/utils/__tests__/humanizeUserFacingError.test.ts`](../frontend-dapp/src/utils/__tests__/humanizeUserFacingError.test.ts) | Regression strings |
| [`frontend-dapp/src/utils/indexerTradeOutageCopy.ts`](../frontend-dapp/src/utils/indexerTradeOutageCopy.ts) | Trade page **market-data-down** banner sentences ([GitLab **#164**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/164), [**#174**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/174)); invariants: [docs/frontend.md § Trade page — indexer outage banner](../docs/frontend.md#trade-page-indexer-outage-banner) |

## Rules of thumb

1. **Prefer the funnel** — apply **`humanizeUserFacingError`** (string) or **`humanizeUserFacingErrorFromUnknown`** (`unknown`) before rendering inline alerts that are not already covered by **`RetryError`** or **`TxResultAlert`**.
2. **`RetryError`** and **`TxResultAlert` (`error`)** already humanize — pass **raw** `Error.message` / `getErrorMessage(query.error)`; do not pre-humanize unless you need a different prefix strategy.
3. **Wallet modal** — `useWalletStore.connect` stores **humanized** text in **`error`**; keep **`console.error`** in wallet services if you need the raw trace.
4. **New patterns** — add try-match branches to **`humanizeOffChainError.ts`** (off-chain) or **`humanizeTerraTxError.ts`** (chain), then extend **`humanizeUserFacingError.test.ts`**.

## Related

- **Decimal amount inputs (block invalid keys before `BigInt`):** [GitLab **#169**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/169), [docs/frontend.md § Decimal amount inputs](../docs/frontend.md#decimal-amount-inputs), [`AGENTS_FRONTEND_DECIMAL_AMOUNT_INPUT.md`](./AGENTS_FRONTEND_DECIMAL_AMOUNT_INPUT.md).
- **LCD / RPC outage (frozen spinner, auto-reconnect):** [GitLab **#171**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/171), [docs/frontend.md § LCD / RPC connectivity](../docs/frontend.md#lcd-rpc-connectivity), [`AGENTS_FRONTEND_LCD_CONNECTIVITY.md`](./AGENTS_FRONTEND_LCD_CONNECTIVITY.md).
- **Wallet broadcast / tx poll hang (offline submit):** [GitLab **#173**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/173), [docs/frontend.md § Transaction broadcast / confirmation timeout](../docs/frontend.md#terra-tx-broadcast-timeout), [`AGENTS_FRONTEND_TX_BROADCAST_TIMEOUT.md`](./AGENTS_FRONTEND_TX_BROADCAST_TIMEOUT.md).
- **Trade page** sub-desktop grid + `trade-sub-lg-workspace`: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md) ([GitLab **#146**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/146)); market-data outage banner vs. book panels: [docs/frontend.md § Trade page — indexer outage banner](../docs/frontend.md#trade-page-indexer-outage-banner) ([GitLab **#164**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/164), [**#174**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/174)).
- **Contract-side copy / max spread:** [GitLab **#134**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/134), [`docs/swap-max-spread-ux.md`](../docs/swap-max-spread-ux.md).
- **Connect modal raw errors (same surface):** [GitLab **#139**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/139); supported extension list / no Leap: [docs/frontend.md § Connect modal: extension install detection](../docs/frontend.md#connect-modal-extension-install) ([GitLab **#159**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/159)).
