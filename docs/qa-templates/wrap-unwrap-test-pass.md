## QA Test Pass — Wrap/Unwrap Flows

**Date:** <!-- YYYY-MM-DD -->
**Tester:** <!-- @gitlab-handle -->
**Environment:** <!-- testnet / mainnet / local -->
**Browser:** <!-- Chrome 130 / Safari 18 / etc. -->
**Wallet:** <!-- Station Extension v4.x -->

---

### Prerequisites
- [ ] Treasury and wrap-mapper contracts deployed
- [ ] LUNC-C and USTC-C CW20 tokens created with wrap-mapper as minter
- [ ] Denom mappings registered (uluna → LUNC-C, uusd → USTC-C)
- [ ] Wrappers registered on treasury (uluna → wrap-mapper, uusd → wrap-mapper)
- [ ] Wrap-mapper set on router (`SetWrapMapper`)
- [ ] Treasury funded with native tokens (≥40M USTC in production)
- [ ] Test wallet has native LUNC and USTC balances

### 1. Direct Wrap (Native → Wrapped CW20)

#### LUNC → LUNC-C
- [ ] Select LUNC as "From" and LUNC-C as "To" on swap page
- [ ] Inline note displays: "This swap will wrap your LUNC (1:1)"
- [ ] Button label remains "Swap" (never changes to "Wrap")
- [ ] Enter amount — estimated output shows 1:1 ratio
- [ ] Execute swap — transaction succeeds
- [ ] LUNC balance decreases by entered amount
- [ ] LUNC-C balance increases by entered amount
- [ ] Treasury LUNC balance increases by entered amount

#### USTC → USTC-C
- [ ] Select USTC as "From" and USTC-C as "To" on swap page
- [ ] Inline note displays: "This swap will wrap your USTC (1:1)"
- [ ] Execute swap — transaction succeeds
- [ ] USTC balance decreases by entered amount
- [ ] USTC-C balance increases by entered amount
- [ ] Treasury USTC balance increases by entered amount

### 2. Direct Unwrap (Wrapped CW20 → Native)

#### LUNC-C → LUNC
- [ ] Select LUNC-C as "From" and LUNC as "To" on swap page
- [ ] Inline note displays: "This swap will unwrap your LUNC-C (1:1)"
- [ ] Button label remains "Swap"
- [ ] Enter amount — estimated output shows 1:1 ratio
- [ ] Execute swap — transaction succeeds
- [ ] LUNC-C balance decreases by entered amount
- [ ] LUNC balance increases by entered amount (minus burn tax if applicable)
- [ ] Treasury LUNC balance decreases by entered amount

#### USTC-C → USTC
- [ ] Select USTC-C as "From" and USTC as "To" on swap page
- [ ] Execute swap — transaction succeeds
- [ ] USTC-C balance decreases by entered amount
- [ ] USTC balance increases by entered amount (minus burn tax if applicable)
- [ ] Treasury USTC balance decreases by entered amount

### 3. Native Input Swap (Native → CW20 via Wrap + Router)

- [ ] Select LUNC as "From" and a non-wrapped CW20 (e.g. EMBER) as "To"
- [ ] Route display shows wrap step (LUNC → LUNC-C → … → EMBER)
- [ ] Inline note: "This swap will wrap your tokens"
- [ ] Enter amount — estimated output updates correctly
- [ ] Execute swap — single transaction with multiple messages succeeds
- [ ] LUNC balance decreases
- [ ] CW20 output token balance increases
- [ ] Treasury LUNC balance increases by input amount

### 4. Native Output Swap (CW20 → Native via Router + Unwrap)

- [ ] Select a CW20 token (e.g. EMBER) as "From" and LUNC as "To"
- [ ] Route display shows unwrap step (EMBER → … → LUNC-C → LUNC)
- [ ] Inline note: "This swap will unwrap your tokens"
- [ ] Execute swap — transaction succeeds with `unwrap_output: true`
- [ ] CW20 input token balance decreases
- [ ] LUNC balance increases
- [ ] Treasury LUNC balance decreases by output amount

### 5. Native-to-Native Swap (Wrap + Router + Unwrap)

- [ ] Select LUNC as "From" and USTC as "To"
- [ ] Route display shows both wrap and unwrap steps
- [ ] Inline note: "This swap will wrap and unwrap your tokens"
- [ ] Execute swap — multi-message transaction succeeds
- [ ] LUNC balance decreases
- [ ] USTC balance increases
- [ ] Treasury LUNC balance increases, Treasury USTC balance decreases

### 6. Treasury Balance Integrity

- [ ] Record treasury LUNC and USTC balances before a sequence of wrap/unwrap operations
- [ ] Perform: 3 wraps, 2 unwraps, 1 native input swap, 1 native output swap
- [ ] After all operations: treasury native balance ≥ total CW20 supply for each denom
- [ ] Query CW20 token_info for LUNC-C and USTC-C — `total_supply` matches expected minted minus burned
- [ ] No "phantom" tokens: every LUNC-C in circulation is backed by LUNC in treasury

### 7. Rate Limits

- [ ] If rate limit is configured for uluna: attempt to wrap more than `max_amount_per_window`
- [ ] Transaction fails with a clear rate-limit error message
- [ ] Wait for the rate limit window to expire, retry — succeeds
- [ ] If rate limit is configured for uusd: same checks as above
- [ ] Rate limit state resets correctly after window expiry
- [ ] Rate limits apply per-denom independently (wrapping LUNC does not affect USTC limit)

### 8. Paused State

- [ ] If wrap-mapper is paused by governance: attempt to wrap → clear "paused" error
- [ ] If wrap-mapper is paused: attempt to unwrap → clear "paused" error
- [ ] Existing CW20 swaps (not involving wrap/unwrap) still work while wrap-mapper is paused

### 9. Error Handling & Edge Cases

- [ ] Wrap with zero amount → blocked or clear error before submission
- [ ] Unwrap with zero amount → blocked or clear error
- [ ] Wrap an unsupported denom (not uluna/uusd) → clear error
- [ ] Unwrap a non-registered CW20 (not LUNC-C/USTC-C) → clear error
- [ ] Unwrap more than treasury holds → transaction reverts with clear error
- [ ] Swap with `unwrap_output: true` but no wrap-mapper set on router → clear error
- [ ] Insufficient native balance for wrap → wallet blocks or clear error
- [ ] Insufficient CW20 balance for unwrap → wallet blocks or clear error
- [ ] Slippage tolerance still enforced on native swaps (minimum_receive)
- [ ] Deadline enforcement still works on native swaps

### 10. Pool UI — Native Token Liquidity

#### Provide Liquidity
- [ ] For a pair containing LUNC-C or USTC-C: "Use native (auto-wrap)" checkbox appears
- [ ] Checking the box: provide liquidity using native LUNC/USTC (auto-wraps in same TX)
- [ ] Transaction succeeds — LP tokens received
- [ ] Treasury balance updated correctly from the wrap

#### Withdraw Liquidity
- [ ] For a pair containing LUNC-C or USTC-C: "Receive as wrapped tokens" checkbox appears
- [ ] With checkbox checked: receive LUNC-C/USTC-C on withdrawal
- [ ] With checkbox unchecked: receive native LUNC/USTC (auto-unwrap)
- [ ] Treasury balance updated correctly from any unwrap

### 11. Token Selector UI

- [ ] Native LUNC appears in the "From" token dropdown
- [ ] Native USTC appears in the "From" token dropdown
- [ ] Native LUNC appears in the "To" token dropdown
- [ ] Native USTC appears in the "To" token dropdown
- [ ] LUNC-C and USTC-C also appear separately (users can choose either)
- [ ] Selecting a native token shows the correct balance (bank query, not CW20)
- [ ] Swap direction toggle (↕) works correctly with native tokens

### 12. Cross-browser / Responsive

- [ ] All wrap/unwrap flows work on Desktop Chrome
- [ ] All wrap/unwrap flows work on Desktop Firefox
- [ ] Wrap/unwrap note text renders correctly on mobile viewports
- [ ] Pool native checkboxes are tappable on mobile

### Notes
<!-- Additional observations, edge cases found, performance issues, etc. -->
