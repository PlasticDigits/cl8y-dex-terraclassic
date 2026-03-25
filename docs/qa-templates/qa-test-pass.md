## QA Test Pass

**Date:** <!-- YYYY-MM-DD -->
**Tester:** <!-- @github-handle -->
**Environment:** <!-- testnet / mainnet / local -->
**Browser:** <!-- Chrome 130 / Safari 18 / etc. -->
**Wallet:** <!-- Station Extension v4.x -->

---

### Wallet Connection
- [ ] Station extension detected
- [ ] Connect button works
- [ ] Wallet address displayed correctly
- [ ] Disconnect works
- [ ] Reconnect after page reload

### Swap Flow
- [ ] Token selection dropdowns populate
- [ ] Typing amount shows estimated output
- [ ] Slippage settings accessible
- [ ] Swap transaction succeeds
- [ ] Balances update after swap
- [ ] Min output exceeded → tx reverts with clear error
- [ ] Zero amount → blocked or clear error

### Pool / Liquidity
- [ ] Pool list loads and shows pairs
- [ ] **Indexer pool list** (only if this release touches pool list or indexer): data from `VITE_INDEXER_URL`; clear error if indexer unavailable (no silent factory fallback)
- [ ] **Indexer pool list (continued):** header shows **total** pair count with thousands separators; search via Apply and Enter; empty search shows unfiltered list; sort, order, and pagination behave correctly
- [ ] **Indexer pool list (continued):** cards show on-chain reserves, fee, Provide/Withdraw; indexed 24h vol (quote) when present; `cd indexer && cargo test --test api_pairs` passes (Postgres + `TEST_DATABASE_URL`)
- [ ] **Indexer pool list (E2E):** `frontend-dapp/e2e/pool.spec.ts` — pair count assertion allows formatted numbers (e.g. thousands separators)
- [ ] Add liquidity — both amounts required
- [ ] Add liquidity — LP tokens received
- [ ] Remove liquidity — LP tokens burned
- [ ] Remove liquidity — both tokens returned
- [ ] Pool reserves update after add/remove

### Create Pair
- [ ] Form accepts two CW20 addresses
- [ ] Validation: same token rejected
- [ ] Validation: non-whitelisted token rejected
- [ ] Pair created successfully
- [ ] New pair appears in pool list

### Responsive / Cross-browser
- [ ] Desktop layout (≥1024px)
- [ ] Tablet layout (768–1023px)
- [ ] Mobile layout (≤767px)
- [ ] No horizontal scroll
- [ ] Touch interactions work (mobile)

### Charts (pair picker — if release touches Charts / indexer pairs)
- [ ] “Find pair” search filters the dropdown (debounced OK)
- [ ] If the selected pair disappears from the filter, selection resets without errors

### General
- [ ] No console errors during normal flows
- [ ] Loading states shown during transactions
- [ ] Error messages are user-friendly
- [ ] Transaction links open block explorer

### Notes
<!-- Additional observations, edge cases found, etc. -->
