# Swap direction amount follow-up

Follow-up to [#1561](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1561),
reported with 10 CL8Y → 5.40572 SpaceUSD. The direction button now starts
5.40572 SpaceUSD as the new exact-input draft, then obtains a separate quote
for SpaceUSD → CL8Y. The same handler serves all viewport sizes.

## Invariants

- Use the eligible current displayed receive raw amount and the old receive
  token's resolved decimals. Preserve every integer digit through plain decimal
  string conversion, including amounts above Number's safe integer range.
- A fresh retained same-intent display can seed the new draft during background
  refresh. This does not reuse old execution data or grant signing permission.
- A missing, revoked, previous-intent, placeholder, non-executable or originally
  expired displayed output clears the new pay field. Check the original deadline
  again when the direction button is activated.
- An explicitly edited positive receive target becomes an exact-input draft
  without waiting for inverse sizing. Clear a draft with nonzero digits beyond
  the token's precision rather than silently rounding it. Zero-only excess
  precision can normalize without changing raw value.
- Clear the old denomination's manual book budget and both quote consent/review
  references. Obtain a quote for the reversed direction and exact new pay size;
  preserve all existing expiry, actor, protection and signing checks.

Implementation: [SwapPage.tsx](../../../frontend-dapp/src/pages/SwapPage.tsx).
Agent guidance: [QL1499](../../../skills/AGENTS_QUOTE_LIFECYCLE.md),
[LQ1452](../../../skills/AGENTS_LOCAL_SWAP_QUOTES.md),
[SL1498](../../../skills/AGENTS_SWAP_LAYOUT.md).

## Verification checklist

- [x] 6-decimal output `5405720` becomes input `5.40572`, with the same exact raw
  value in the reversed quote request.
- [x] Mixed 6/18-decimal assets scale the new pay field using its own decimals.
- [x] Full 18-decimal precision and an amount above 2^53 retain every raw digit.
- [x] An unresolved output clears the new input instead of carrying the old 10.
- [x] A draft edited before debounce cannot adopt the previous amount's output.
- [x] A fresh same-intent output survives a background refresh and seeds the flip.
- [x] An original deadline crossed before the click prevents adoption.
- [x] An edited receive target starts an exact-input request for the reversed pair.
- [x] Excess nonzero token precision clears rather than rounds.
- [x] Generated amounts longer than the ordinary 24-character draft limit allow
  backspace and same-length digit corrections without allowing further growth.
- [x] Live production browser checks on desktop and phone-sized viewports.
  Both adopted `0.851568201759174334` as new pay and requested raw
  `851568201759174334` for the reversed quote (HTTP200), with zero signing.
  See [live receipt](status-direction-live.json).

The focused component suite is
`src/pages/SwapPage.assetDecimals.test.tsx`: 19 tests passed in 7.28 seconds. An earlier bounded
run also included seam and existing URL/direction regressions: 33 tests passed,
49 unrelated tests filtered out, in 7.34 seconds on 2026-10-19 at 06:00:49 UTC.
The old exact-output-preserving flip assertion was updated to require the new
exact-input URL intent. These tests validate UI intent and local request binding;
they do not establish funded settlement or physical-device acceptance.

```sh
timeout 55s bash scripts/with-node.sh --cwd frontend-dapp -- \
  ./node_modules/.bin/vitest run src/pages/SwapPage.assetDecimals.test.tsx \
  src/swapDirectionSeam.test.ts src/pages/SwapPage.queryParams.test.tsx \
  --testNamePattern='switch|flip|Swap direction seam|T[1-8]:|hostile indexed'
```
