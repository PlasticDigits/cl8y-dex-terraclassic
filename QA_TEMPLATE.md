# CL8Y DEX – Comprehensive QA Testing Template

> **Version**: 1.0  
> **Date**: ___________  
> **Tester**: ___________  
> **Network**: [ ] Local  [ ] Testnet (rebel-2)  [ ] Mainnet (columbus-5)  
> **Browser**: ___________  
> **OS**: ___________  
> **Build/Commit**: ___________

---

## Status Legend

| Symbol | Meaning |
|--------|---------|
| ✅ | Pass |
| ❌ | Fail |
| ⚠️ | Pass with issues |
| ⏭️ | Skipped (with reason) |
| 🔄 | Blocked / Needs retest |

---

## 1. WALLET CONNECTION & MANAGEMENT

### 1.1 Station Wallet (Extension)

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 1.1.1 | Connect Station | Click wallet button → Select Station | Modal opens, wallet connects, truncated address shown in header | | |
| 1.1.2 | Station not installed | Remove/disable Station extension → Click wallet button → Select Station | User-friendly "not installed" error message displayed | | |
| 1.1.3 | Station reject connection | Click Station → Reject in extension popup | "Transaction rejected by user" or equivalent message | | |
| 1.1.4 | Station disconnect | Click connected address → Click Disconnect | Wallet disconnects, UI reverts to "Connect Wallet" state | | |
| 1.1.5 | Station reconnect on refresh | Connect Station → Refresh page | Wallet auto-reconnects, address persists | | |
| 1.1.6 | Station chain mismatch | Connect with wrong chain selected | "Chain not found" error message | | |
| 1.1.7 | Station swap tx signing | Initiate swap → Sign in Station | Transaction signed and broadcast successfully | | |
| 1.1.8 | Station swap tx rejection | Initiate swap → Reject in Station | "Transaction rejected by user" message, UI returns to ready state | | |
| 1.1.9 | Station provide liquidity signing | Provide liquidity → Sign all messages | Both allowance + provide_liquidity msgs signed and executed | | |
| 1.1.10 | Station withdraw liquidity signing | Withdraw LP → Sign in Station | Transaction succeeds, LP balance updates | | |
| 1.1.11 | Station create pair signing | Create pair → Sign in Station | Pair creation tx broadcast, success feedback | | |
| 1.1.12 | Station fee tier register signing | Register for tier → Sign in Station | Registration tx succeeds | | |

### 1.2 Keplr Wallet (Extension)

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 1.2.1 | Connect Keplr | Click wallet button → Select Keplr | Wallet connects, address shown | | |
| 1.2.2 | Keplr not installed | Disable Keplr → Select Keplr | "Not installed" error | | |
| 1.2.3 | Keplr reject connection | Select Keplr → Reject | Appropriate error message | | |
| 1.2.4 | Keplr disconnect | Connected → Disconnect | Clean disconnect, UI resets | | |
| 1.2.5 | Keplr swap tx signing | Initiate swap → Sign in Keplr | Success | | |
| 1.2.6 | Keplr swap tx rejection | Initiate swap → Reject in Keplr | Rejection message, clean state | | |
| 1.2.7 | Keplr provide liquidity | Provide liquidity → Sign all | Success | | |
| 1.2.8 | Keplr withdraw liquidity | Withdraw LP → Sign | Success | | |
| 1.2.9 | Keplr create pair | Create pair → Sign | Success | | |
| 1.2.10 | Keplr fee tier register | Register → Sign | Success | | |

### 1.3 Leap Wallet (Extension)

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 1.3.1 | Connect Leap | Click wallet button → Select Leap | Wallet connects, address shown | | |
| 1.3.2 | Leap not installed | Disable Leap → Select Leap | "Not installed" error | | |
| 1.3.3 | Leap reject connection | Select Leap → Reject | Appropriate error message | | |
| 1.3.4 | Leap disconnect | Connected → Disconnect | Clean disconnect | | |
| 1.3.5 | Leap swap tx signing | Initiate swap → Sign in Leap | Success | | |
| 1.3.6 | Leap swap tx rejection | Initiate swap → Reject | Rejection message | | |
| 1.3.7 | Leap provide liquidity | Provide → Sign all | Success | | |
| 1.3.8 | Leap withdraw liquidity | Withdraw → Sign | Success | | |
| 1.3.9 | Leap create pair | Create → Sign | Success | | |
| 1.3.10 | Leap fee tier register | Register → Sign | Success | | |

### 1.4 Cosmostation Wallet (Extension)

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 1.4.1 | Connect Cosmostation | Click wallet button → Select Cosmostation | Wallet connects, address shown | | |
| 1.4.2 | Cosmostation not installed | Disable → Select Cosmostation | "Not installed" error | | |
| 1.4.3 | Cosmostation reject | Select → Reject | Error message | | |
| 1.4.4 | Cosmostation disconnect | Connected → Disconnect | Clean disconnect | | |
| 1.4.5 | Cosmostation swap signing | Swap → Sign | Success | | |
| 1.4.6 | Cosmostation swap rejection | Swap → Reject | Rejection message | | |
| 1.4.7 | Cosmostation provide liquidity | Provide → Sign all | Success | | |
| 1.4.8 | Cosmostation withdraw liquidity | Withdraw → Sign | Success | | |
| 1.4.9 | Cosmostation create pair | Create → Sign | Success | | |
| 1.4.10 | Cosmostation fee tier register | Register → Sign | Success | | |

### 1.5 LuncDash (WalletConnect – Mobile)

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 1.5.1 | Connect LuncDash | Click wallet → Select LuncDash → Scan QR | WalletConnect session established, address shown | | |
| 1.5.2 | LuncDash QR timeout | Show QR → Wait without scanning | Timeout handled gracefully | | |
| 1.5.3 | LuncDash reject connection | Scan QR → Reject on mobile | Error message displayed | | |
| 1.5.4 | LuncDash disconnect | Connected → Disconnect | Session closed, UI resets | | |
| 1.5.5 | LuncDash 0-wallet edge case | Connect when controller returns 0 wallets | LCD account check + pub key validation occurs, appropriate error or recovery | | |
| 1.5.6 | LuncDash swap signing | Swap → Approve on mobile | Transaction succeeds | | |
| 1.5.7 | LuncDash swap rejection | Swap → Reject on mobile | Rejection message | | |
| 1.5.8 | LuncDash provide liquidity | Provide → Approve all msgs | Success | | |
| 1.5.9 | LuncDash withdraw liquidity | Withdraw → Approve | Success | | |
| 1.5.10 | LuncDash session persistence | Connect → Close browser tab → Reopen | Session resumes or prompts reconnect | | |

### 1.6 Galaxy Station (WalletConnect – Mobile)

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 1.6.1 | Connect Galaxy Station | Click wallet → Select Galaxy Station → Scan QR | Session established, address shown | | |
| 1.6.2 | Galaxy Station QR timeout | Show QR → Wait | Graceful timeout | | |
| 1.6.3 | Galaxy Station reject | Scan → Reject on mobile | Error message | | |
| 1.6.4 | Galaxy Station disconnect | Connected → Disconnect | Session closed, UI resets | | |
| 1.6.5 | Galaxy Station 0-wallet edge case | Controller returns 0 wallets | LCD check + pub key validation | | |
| 1.6.6 | Galaxy Station swap signing | Swap → Approve | Success | | |
| 1.6.7 | Galaxy Station swap rejection | Swap → Reject | Rejection message | | |
| 1.6.8 | Galaxy Station provide liquidity | Provide → Approve | Success | | |
| 1.6.9 | Galaxy Station withdraw liquidity | Withdraw → Approve | Success | | |
| 1.6.10 | Galaxy Station session persistence | Connect → Close/reopen browser | Session resumes or reconnects | | |

### 1.7 Simulated Wallet (Dev Mode Only)

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 1.7.1 | Simulated wallet visible in dev mode | Set VITE_DEV_MODE=true → Open wallet modal | "Simulated Wallet" option appears | | |
| 1.7.2 | Simulated wallet hidden in production | VITE_DEV_MODE=false → Open wallet modal | "Simulated Wallet" not shown | | |
| 1.7.3 | Connect simulated wallet | Select Simulated Wallet | Connects with fixed address terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v | | |
| 1.7.4 | Simulated wallet swap | Connect simulated → Execute swap | Transaction succeeds using dev mnemonic | | |
| 1.7.5 | Custom dev mnemonic | Set VITE_DEV_MNEMONIC → Connect | Uses custom mnemonic | | |

### 1.8 Wallet Modal UX

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 1.8.1 | Modal opens on click | Click "Connect Wallet" button | Modal opens with all wallet options | | |
| 1.8.2 | Modal closes on backdrop click | Open modal → Click outside | Modal closes | | |
| 1.8.3 | Modal closes on X button | Open modal → Click X | Modal closes | | |
| 1.8.4 | Modal closes on ESC | Open modal → Press Escape | Modal closes | | |
| 1.8.5 | Connected state display | Connect any wallet | Truncated address, Terra Classic icon, dropdown arrow shown | | |
| 1.8.6 | Dropdown disconnect | Click connected address → Disconnect option | Dropdown appears, disconnect works | | |
| 1.8.7 | Only one wallet at a time | Connect wallet A → Connect wallet B | First disconnects, second connects | | |

---

## 2. SWAP FUNCTIONALITY

### 2.1 Swap Core Flow

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 2.1.1 | Basic swap | Select pair → Enter amount → Swap | Simulation shows receive amount, swap executes, success alert with tx hash | | |
| 2.1.2 | Pair selection | Click pair dropdown → Select different pair | Pair changes, reserves update, simulation clears | | |
| 2.1.3 | Amount input validation | Enter negative / 0 / text / special chars | Only valid positive numbers accepted | | |
| 2.1.4 | Very small amount | Enter minimum possible amount (e.g., 0.000001) | Simulation runs, or appropriate minimum amount error | | |
| 2.1.5 | Very large amount | Enter amount exceeding wallet balance | UI prevents swap or shows insufficient balance | | |
| 2.1.6 | Amount exceeding pool liquidity | Enter amount larger than pool reserves | High price impact warning, simulation may fail or show extreme slippage | | |
| 2.1.7 | Direction toggle | Click swap direction arrow | Offer and receive assets swap, amounts recalculate | | |
| 2.1.8 | Direction toggle with amount | Enter amount → Toggle direction | Simulation re-runs with new direction | | |
| 2.1.9 | Clear amount on pair change | Enter amount → Change pair | Amount clears or simulation re-runs | | |
| 2.1.10 | Swap with no wallet | Enter amount without wallet connected | Button shows "Connect Wallet" | | |
| 2.1.11 | Swap with no pair selected | Connect wallet, no pair selected | Button shows "Select a Pair" | | |
| 2.1.12 | Swap with no amount | Connect wallet, select pair, empty amount | Button shows "Enter Amount" | | |

### 2.2 Swap Simulation

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 2.2.1 | Simulation accuracy | Enter amount → Note simulated return → Execute swap | Actual return within slippage tolerance of simulation | | |
| 2.2.2 | Simulation loading state | Enter amount quickly | "Calculating…" shown during simulation query | | |
| 2.2.3 | Simulation updates on amount change | Change amount repeatedly | Simulation re-runs on each change (debounced) | | |
| 2.2.4 | Simulation failure | Enter amount when pair contract is unresponsive | Error state shown, swap button disabled | | |
| 2.2.5 | Pool reserves display | Select pair → View trade details | Pool reserves shown for both assets | | |
| 2.2.6 | Price display | Complete simulation | Price shown in trade details | | |

### 2.3 Slippage Settings

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 2.3.1 | Default slippage | Open swap page fresh | Slippage is 0.5% | | |
| 2.3.2 | Preset 0.1% | Click 0.1% button | Slippage set to 0.1%, min received recalculates | | |
| 2.3.3 | Preset 0.5% | Click 0.5% button | Slippage set to 0.5% | | |
| 2.3.4 | Preset 1% | Click 1% button | Slippage set to 1% | | |
| 2.3.5 | Custom slippage | Enter custom value (e.g., 2.5%) | Custom slippage applied | | |
| 2.3.6 | Maximum slippage | Enter 50% | Accepted (max) | | |
| 2.3.7 | Exceed max slippage | Enter 51% | Rejected or capped at 50% | | |
| 2.3.8 | Zero slippage | Enter 0% | Allowed but likely to fail on execution | | |
| 2.3.9 | Negative slippage | Enter -1% | Rejected | | |
| 2.3.10 | Slippage persistence | Set slippage → Navigate away → Return | Slippage persists (Zustand store) | | |
| 2.3.11 | Min received calculation | Set slippage to 1%, note simulation return | Min received = return × (1 - 0.01) | | |
| 2.3.12 | Slippage vs execution | Set very low slippage (0.01%) → Swap | Transaction may fail due to price movement; error handled | | |

### 2.4 Price Impact

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 2.4.1 | Low price impact (<1%) | Swap small amount relative to pool | Green price impact indicator | | |
| 2.4.2 | Medium price impact (1-5%) | Swap moderate amount | Amber/yellow price impact indicator | | |
| 2.4.3 | High price impact (>5%) | Swap large amount | Red price impact, "High price impact!" warning | | |
| 2.4.4 | Price impact accuracy | Compare displayed impact vs manual calculation | Values match within rounding tolerance | | |

### 2.5 Swap Fee Display

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 2.5.1 | Base fee display | Select pair → View trade details | Fee shown (e.g., 1.8%) | | |
| 2.5.2 | Fee with no discount | Swap without fee tier registration | Full base fee applied | | |
| 2.5.3 | Fee with discount | Register for tier → Swap | Original fee struck through, effective fee in cyan, discount % shown | | |
| 2.5.4 | Commission amount | Execute swap → View details | Commission amount displayed | | |
| 2.5.5 | Fee CTA for unregistered | Connect wallet, not registered | "Hold CL8Y to reduce swap fees →" link to /tiers | | |
| 2.5.6 | Fee CTA link works | Click fee CTA | Navigates to /tiers page | | |

### 2.6 Swap Transaction States

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 2.6.1 | Success alert | Complete successful swap | Green alert-success with tx hash | | |
| 2.6.2 | Error alert | Swap fails (network error, etc.) | Red alert-error with message | | |
| 2.6.3 | Loading state during swap | Click Swap → Wait | Button shows "Swapping…", inputs disabled | | |
| 2.6.4 | User rejection | Click Swap → Reject in wallet | "Transaction rejected by user" message, UI returns to ready | | |
| 2.6.5 | Network error during swap | Disconnect internet → Swap | Network error message displayed | | |
| 2.6.6 | Tx hash clickable | Success alert → Click tx hash | Opens block explorer with tx details | | |

---

## 3. LIQUIDITY POOLS

### 3.1 Pool Display

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 3.1.1 | Pool page loads | Navigate to /pool | Pool cards displayed for available pairs | | |
| 3.1.2 | Pool card info | View any pool card | Pair info, fee badge, pool reserves for both assets shown | | |
| 3.1.3 | Fee badge with discount | User has fee tier → View pool card | Discounted fee shown | | |
| 3.1.4 | Pool reserves accuracy | Compare on-chain reserves vs displayed | Values match | | |
| 3.1.5 | Tab switching | Click Provide/Withdraw tabs | Tabs switch content correctly | | |

### 3.2 Provide Liquidity

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 3.2.1 | Provide liquidity basic | Enter amounts for both assets → Provide | increase_allowance (×2) + provide_liquidity executed, LP tokens received | | |
| 3.2.2 | Amount input Asset A | Enter amount for Asset A | Valid number accepted | | |
| 3.2.3 | Amount input Asset B | Enter amount for Asset B | Valid number accepted | | |
| 3.2.4 | Zero amount | Enter 0 for either asset | Prevented or error | | |
| 3.2.5 | Exceed balance | Enter more than wallet balance | Prevented or insufficient balance error | | |
| 3.2.6 | Allowance flow | Provide → Watch tx sequence | Two increase_allowance txs, then provide_liquidity tx | | |
| 3.2.7 | Allowance rollback on failure | provide_liquidity fails after allowances set | decrease_allowance called on both tokens | | |
| 3.2.8 | LP token receipt | Successfully provide | LP token balance increases | | |
| 3.2.9 | Pool reserves update | Provide liquidity → Check reserves | Pool reserves increased by provided amounts | | |
| 3.2.10 | Provide without wallet | Try to provide without connecting | "Connect Wallet" prompt | | |
| 3.2.11 | Provide success feedback | Complete provision | Success message/alert shown | | |
| 3.2.12 | Provide error feedback | Transaction fails | Error message shown | | |

### 3.3 Withdraw Liquidity

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 3.3.1 | Withdraw basic | Enter LP amount → Withdraw | send LP to pair with withdraw_liquidity msg, assets returned | | |
| 3.3.2 | Withdraw all LP | Enter full LP balance | All liquidity withdrawn | | |
| 3.3.3 | Withdraw partial | Enter partial LP amount | Proportional assets returned | | |
| 3.3.4 | Zero LP amount | Enter 0 | Prevented or error | | |
| 3.3.5 | Exceed LP balance | Enter more than LP balance | Prevented or error | | |
| 3.3.6 | Withdraw without LP tokens | User has no LP tokens | Appropriate message or disabled | | |
| 3.3.7 | Pool reserves after withdraw | Withdraw → Check reserves | Reserves decreased proportionally | | |
| 3.3.8 | Withdraw success feedback | Complete withdrawal | Success alert | | |
| 3.3.9 | Withdraw error feedback | Transaction fails | Error alert | | |

---

## 4. FEE TIERS & DISCOUNT SYSTEM

### 4.1 Tiers Page Display

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 4.1.1 | Tiers page loads | Navigate to /tiers | Tier list displayed | | |
| 4.1.2 | Tier list content | View tier table | Each tier shows: ID, CL8Y requirement, discount %, effective fee | | |
| 4.1.3 | Base fee display | View tiers | Base fee shown as 1.8% (180 bps) | | |
| 4.1.4 | Effective fee calculation | Check any tier | Effective fee = 1.8% × (1 - discount/10000) | | |
| 4.1.5 | "How it works" section | Scroll down on tiers page | Explanation section visible and accurate | | |
| 4.1.6 | Your Status - not registered | Connect wallet, not registered | Shows "Not registered" | | |
| 4.1.7 | Your Status - registered | Connect registered wallet | Shows current tier name | | |
| 4.1.8 | Your Status - no wallet | View without wallet | Appropriate message or hidden | | |

### 4.2 Tier Registration

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 4.2.1 | Register for self-register tier | Hold required CL8Y → Register | Registration tx succeeds, status updates | | |
| 4.2.2 | Register insufficient CL8Y | Hold less CL8Y than required → Register | Registration fails with appropriate error | | |
| 4.2.3 | Register governance-only tier | Try to register for governance tier | Self-register button not available or disabled | | |
| 4.2.4 | Deregister from tier | Registered → Deregister | Deregistration succeeds, status reverts | | |
| 4.2.5 | Deregister governance tier | Try to deregister from governance tier | Not allowed (no button or disabled) | | |
| 4.2.6 | Register without wallet | Try to register without wallet | "Connect Wallet" prompt | | |
| 4.2.7 | Tier upgrade | Register lower tier → Get more CL8Y → Register higher | Tier updates correctly | | |
| 4.2.8 | Market Maker tier (ID 0) | Check governance tier | Labeled correctly, no self-register | | |
| 4.2.9 | Blacklist tier | Check blacklist tier | Labeled correctly, no self-register | | |

### 4.3 Fee Discount Application

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 4.3.1 | Discount on swap page | Register for tier → Go to swap | Discounted fee shown on swap | | |
| 4.3.2 | Discount on pool page | Register for tier → Go to pool | Discounted fee shown on pool card | | |
| 4.3.3 | Discount visual | View fee with discount | Original fee struck through, effective fee in cyan | | |
| 4.3.4 | Discount percentage shown | View fee with discount | Discount % displayed next to effective fee | | |
| 4.3.5 | Discount applied to tx | Execute swap with discount | Commission in tx matches discounted fee, not base fee | | |
| 4.3.6 | No discount applied | Swap without registration | Full base fee charged | | |

---

## 5. CHARTS & ANALYTICS

### 5.1 Price Chart

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 5.1.1 | Chart loads | Navigate to /charts → Select pair | Candlestick chart renders with OHLC data | | |
| 5.1.2 | 1-minute candles | Select 1m interval | 1m candles displayed | | |
| 5.1.3 | 5-minute candles | Select 5m interval | 5m candles displayed | | |
| 5.1.4 | 15-minute candles | Select 15m interval | 15m candles displayed | | |
| 5.1.5 | 1-hour candles | Select 1h interval | 1h candles displayed | | |
| 5.1.6 | 4-hour candles | Select 4h interval | 4h candles displayed | | |
| 5.1.7 | 1-day candles | Select 1d interval | 1d candles displayed | | |
| 5.1.8 | Candle colors | View chart with up and down candles | Green for up, red for down | | |
| 5.1.9 | Chart pair switching | Switch pair in selector | Chart reloads with new pair data | | |
| 5.1.10 | Chart loading state | Switch pair/interval | Loading indicator shown during data fetch | | |
| 5.1.11 | Chart error state | Indexer unavailable | "Failed to load chart data" message | | |
| 5.1.12 | Chart zoom/scroll | Zoom in/out, scroll left/right | Chart responds to lightweight-charts interactions | | |
| 5.1.13 | Chart with no data | Select pair with no trades | Empty chart or "No data" message | | |
| 5.1.14 | OHLCV accuracy | Compare candle data with known trades | Open/High/Low/Close/Volume match expected values | | |

### 5.2 24-Hour Statistics

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 5.2.1 | 24h volume (base) | View pair stats | Base volume displayed and accurate | | |
| 5.2.2 | 24h volume (quote) | View pair stats | Quote volume displayed and accurate | | |
| 5.2.3 | 24h trade count | View pair stats | Trade count matches actual 24h trades | | |
| 5.2.4 | 24h price change % | View pair stats | Price change % calculated correctly | | |
| 5.2.5 | 24h high | View pair stats | Correct highest price in 24h | | |
| 5.2.6 | 24h low | View pair stats | Correct lowest price in 24h | | |
| 5.2.7 | 24h open | View pair stats | Correct opening price 24h ago | | |
| 5.2.8 | 24h close | View pair stats | Correct most recent price | | |
| 5.2.9 | Stats with no recent trades | View pair with no 24h activity | Zeros or "No activity" shown | | |

### 5.3 Overview Statistics

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 5.3.1 | Global 24h volume | View overview section | Total 24h volume across all pairs | | |
| 5.3.2 | Global 24h trades | View overview section | Total 24h trade count | | |
| 5.3.3 | Pair count | View overview section | Correct number of active pairs | | |
| 5.3.4 | Token count | View overview section | Correct number of unique tokens | | |

### 5.4 Recent Trades

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 5.4.1 | Recent trades load | Select pair on charts | Trade table shows recent trades | | |
| 5.4.2 | Trade time | View trade entry | Timestamp displayed correctly | | |
| 5.4.3 | Trade direction | View trade entry | Buy/Sell direction shown | | |
| 5.4.4 | Trade offer amount | View trade entry | Offer amount with token symbol | | |
| 5.4.5 | Trade return amount | View trade entry | Return amount with token symbol | | |
| 5.4.6 | Trade price | View trade entry | Price calculation correct | | |
| 5.4.7 | Trade tx hash | View trade entry | Tx hash shown, clickable | | |
| 5.4.8 | Tx hash link | Click tx hash | Opens Terra Classic explorer | | |
| 5.4.9 | No trades state | Pair with no trades | "No trades yet" message | | |
| 5.4.10 | Pagination/load more | Pair with many trades | Cursor-based pagination works (before param) | | |

### 5.5 Leaderboard

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 5.5.1 | Leaderboard loads | View leaderboard section | Trader list displayed | | |
| 5.5.2 | Sort by Volume | Select Volume sort | Traders sorted by total volume DESC | | |
| 5.5.3 | Sort by Best Trade | Select Best Trade sort | Traders sorted by best trade P&L DESC | | |
| 5.5.4 | Sort by Most Profit | Select Most Profit sort | Traders sorted by total realized P&L DESC | | |
| 5.5.5 | Sort by Most Loss | Select Most Loss sort | Traders sorted by worst P&L DESC | | |
| 5.5.6 | Trader link | Click trader in leaderboard | Navigates to /trader/:address | | |
| 5.5.7 | No traders state | No trades indexed | "No traders yet" message | | |
| 5.5.8 | Leaderboard limit | Over 200 traders | Max 200 shown | | |

---

## 6. TRADER PROFILE

### 6.1 Profile Access

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 6.1.1 | Direct URL access | Navigate to /trader/terra1abc... | Profile loads for that address | | |
| 6.1.2 | Search by address | Enter address in search → Click Search | Profile loads | | |
| 6.1.3 | My Profile link | Connect wallet → Click "My Profile" | Navigates to /trader/{connected_address} | | |
| 6.1.4 | Invalid address search | Enter invalid address → Search | Error or "not found" message | | |
| 6.1.5 | Non-trader address | Enter address that never traded → Search | "Trader not found. They may not have traded yet." | | |
| 6.1.6 | No address state | Visit /trader without address | "Enter a trader address above or connect your wallet…" | | |

### 6.2 Profile Content

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 6.2.1 | Address display | View profile | Full address shown | | |
| 6.2.2 | "You" badge | View own profile | "You" badge displayed | | |
| 6.2.3 | "You" badge absent | View someone else's profile | "You" badge not shown | | |
| 6.2.4 | Tier name display | View profile with tier | Tier name shown | | |
| 6.2.5 | Total trades | View stats | Correct total trade count | | |
| 6.2.6 | Total volume | View stats | Correct total volume | | |
| 6.2.7 | First trade date | View stats | Correct first trade timestamp | | |
| 6.2.8 | Last trade date | View stats | Correct last trade timestamp | | |

### 6.3 P&L and Positions

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 6.3.1 | Total realized P&L | View P&L section | Correct sum of all realized P&L | | |
| 6.3.2 | Best trade P&L | View P&L section | Correct best single trade | | |
| 6.3.3 | Worst trade P&L | View P&L section | Correct worst single trade | | |
| 6.3.4 | Total fees paid | View P&L section | Correct sum of all fees (spread + commission) | | |
| 6.3.5 | Position per pair | View positions table | Each pair shows: pair, net position, avg entry, cost basis, realized P&L, trade count | | |
| 6.3.6 | Net position accuracy | Compare with manual calculation | Net position matches buy - sell accumulation | | |
| 6.3.7 | Avg entry price accuracy | Compare with manual calculation | Weighted average entry price correct | | |
| 6.3.8 | Realized P&L per position | Compare with manual calculation | (exit_price - avg_entry) × amount for each sell | | |
| 6.3.9 | No positions | Trader with no active positions | Empty or "No positions" shown | | |

### 6.4 Trade History

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 6.4.1 | Trade history loads | View trader profile | Trade history table displayed | | |
| 6.4.2 | Trade details | View any trade row | Time, direction, offer, return, price, tx hash | | |
| 6.4.3 | Trade pagination | Trader with many trades | Load more / cursor pagination works | | |
| 6.4.4 | Trade tx hash link | Click tx hash in history | Opens block explorer | | |

---

## 7. CREATE PAIR

### 7.1 Create Pair Flow

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 7.1.1 | Page loads | Navigate to /create | Create pair form displayed | | |
| 7.1.2 | Valid token addresses | Enter two valid terra1... CW20 addresses | Addresses accepted, validation passes | | |
| 7.1.3 | Invalid address format | Enter non-terra1 address | Validation error shown | | |
| 7.1.4 | Same token addresses | Enter same address for both | "Token addresses must be different" error | | |
| 7.1.5 | Whitelisted code IDs check | Enter CW20 with whitelisted code ID | No warning shown | | |
| 7.1.6 | Non-whitelisted code ID | Enter CW20 with non-whitelisted code | Warning: "not whitelisted", "transaction likely to fail" | | |
| 7.1.7 | Code ID query failure | Enter address that can't be queried | "Could not query contract info" message | | |
| 7.1.8 | Duplicate pair creation | Enter tokens for existing pair | Error or "pair already exists" | | |
| 7.1.9 | Successful pair creation | Enter valid tokens → Sign tx | create_pair on factory, success feedback | | |
| 7.1.10 | Create without wallet | Try to create without connecting | "Connect Wallet" prompt | | |
| 7.1.11 | Info box content | View page | CW20 info, whitelisted code IDs, pair requirements shown | | |

---

## 8. TOKEN DISPLAY & RESOLUTION

### 8.1 Token Information

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 8.1.1 | CW20 token symbol | View any CW20 token in UI | Symbol fetched from token_info and displayed | | |
| 8.1.2 | CW20 token logo | View CW20 token | Blockies identicon shown | | |
| 8.1.3 | LUNC display | View uluna denomination | "LUNC" symbol displayed | | |
| 8.1.4 | USTC display | View uusd denomination | "USTC" symbol displayed | | |
| 8.1.5 | Unknown CW20 initial load | View unknown CW20 before info fetched | Shortened address shown temporarily | | |
| 8.1.6 | Token info caching | Load token → Check localStorage → Reload | Token info cached in cl8y-dex-token-info, no re-fetch | | |
| 8.1.7 | Token display in swap | View token in swap card | Logo + symbol displayed correctly | | |
| 8.1.8 | Token display in pool | View token in pool card | Logo + symbol displayed correctly | | |
| 8.1.9 | Token display in charts | View token in chart pair selector | Correct symbols shown | | |

---

## 9. NAVIGATION & ROUTING

### 9.1 Page Navigation

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 9.1.1 | Swap page (/) | Click Swap nav | Swap page loads | | |
| 9.1.2 | Pool page (/pool) | Click Pool nav | Pool page loads | | |
| 9.1.3 | Charts page (/charts) | Click Charts nav | Charts page loads | | |
| 9.1.4 | Fee Tiers page (/tiers) | Click Fee Tiers nav | Tiers page loads | | |
| 9.1.5 | Create Pair page (/create) | Click Create Pair nav | Create page loads | | |
| 9.1.6 | Trader page (/trader/:addr) | Navigate to URL | Trader profile loads | | |
| 9.1.7 | Unknown route redirect | Navigate to /nonexistent | Redirects to / | | |
| 9.1.8 | Browser back/forward | Navigate pages → Use back/forward | Correct page shown | | |
| 9.1.9 | Direct URL access | Type /charts in address bar | Charts page loads directly | | |
| 9.1.10 | Lazy loading | Navigate to each page first time | Pages lazy-load without errors | | |

### 9.2 Header & Footer

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 9.2.1 | Logo display | View header | CL8Y DEX logo shown | | |
| 9.2.2 | Nav items | View navigation | Swap, Pool, Charts, Fee Tiers, Create Pair visible | | |
| 9.2.3 | Nav icons + labels | View nav items | Each has icon and label | | |
| 9.2.4 | Active nav highlight | Navigate to page | Current page nav item highlighted | | |
| 9.2.5 | Footer content | View footer | "CL8Y DEX · Terra Classic" text | | |
| 9.2.6 | Footer theme toggle | View footer | Dark/Light theme toggle present | | |

---

## 10. THEME & UI/UX

### 10.1 Theme System

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 10.1.1 | Default theme | Fresh visit (no localStorage) | Follows system preference | | |
| 10.1.2 | Dark mode | Toggle to dark | Dark theme applied across all pages | | |
| 10.1.3 | Light mode | Toggle to light | Light theme applied across all pages | | |
| 10.1.4 | Theme persistence | Set theme → Reload | Theme persists via localStorage (cl8y-dex-theme) | | |
| 10.1.5 | Theme toggle | Click toggle in footer | Theme switches immediately | | |
| 10.1.6 | Dark mode readability | Browse all pages in dark mode | All text readable, no invisible elements | | |
| 10.1.7 | Light mode readability | Browse all pages in light mode | All text readable, no invisible elements | | |

### 10.2 Visual Design

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 10.2.1 | Neo-brutalist borders | View components | Borders and shadows match design system | | |
| 10.2.2 | Fonts - headings | View headings | Chakra Petch font | | |
| 10.2.3 | Fonts - body | View body text | IBM Plex Sans font | | |
| 10.2.4 | Uppercase labels | View labels | Uppercase styling applied | | |
| 10.2.5 | Custom cursors - default | Move mouse on page | Custom cursor-default.png | | |
| 10.2.6 | Custom cursors - pointer | Hover on clickable elements | Custom cursor image | | |
| 10.2.7 | Custom cursors - wait | During loading states | Wait cursor shown | | |
| 10.2.8 | Custom cursors - text | Hover on text inputs | Text cursor shown | | |
| 10.2.9 | Custom cursors - not-allowed | Hover on disabled elements | Not-allowed cursor | | |
| 10.2.10 | Custom cursors - grab | Hover on draggable elements | Grab cursor | | |
| 10.2.11 | Custom cursors - grabbing | Dragging elements | Grabbing cursor | | |

### 10.3 Sound Effects

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 10.3.1 | Button press sound | Click a button | Press sound plays | | |
| 10.3.2 | Hover sound | Hover over interactive element | Hover sound plays | | |
| 10.3.3 | Success sound | Complete a successful swap | Success sound plays | | |
| 10.3.4 | Error sound | Encounter an error | Error sound plays | | |

---

## 11. RESPONSIVE DESIGN & MOBILE

### 11.1 Mobile Layout

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 11.1.1 | Swap page mobile | View /swap on mobile viewport | Layout adapts, all elements usable | | |
| 11.1.2 | Pool page mobile | View /pool on mobile | Layout adapts | | |
| 11.1.3 | Charts page mobile | View /charts on mobile | Chart renders, controls accessible | | |
| 11.1.4 | Tiers page mobile | View /tiers on mobile | Table readable, buttons accessible | | |
| 11.1.5 | Create pair mobile | View /create on mobile | Form usable | | |
| 11.1.6 | Trader page mobile | View /trader on mobile | All sections visible and readable | | |
| 11.1.7 | Navigation mobile | View nav on mobile | Mobile-friendly navigation | | |
| 11.1.8 | Wallet modal mobile | Open wallet modal on mobile | Modal fits screen, all options visible | | |
| 11.1.9 | Touch interactions | Tap, swipe on mobile | All interactions work with touch | | |

### 11.2 Breakpoints

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 11.2.1 | 320px width | Resize to 320px | No horizontal scroll, content readable | | |
| 11.2.2 | 375px width (iPhone SE) | Resize to 375px | Layout correct | | |
| 11.2.3 | 768px width (tablet) | Resize to 768px | Layout correct | | |
| 11.2.4 | 1024px width (small desktop) | Resize to 1024px | Layout correct | | |
| 11.2.5 | 1440px width (desktop) | Resize to 1440px | Layout correct | | |
| 11.2.6 | 1920px+ (large desktop) | Resize to 1920px | Content doesn't stretch excessively | | |

---

## 12. ERROR HANDLING & EDGE CASES

### 12.1 Error Boundary

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 12.1.1 | Render error caught | Trigger a component render error | "Something went wrong" displayed with "Reload App" button | | |
| 12.1.2 | Reload App button | Click "Reload App" after error | Page reloads | | |

### 12.2 Network Errors

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 12.2.1 | LCD unreachable | Disconnect/block LCD URL | Appropriate network error messages | | |
| 12.2.2 | Indexer unreachable | Block indexer URL | Charts/stats show error states | | |
| 12.2.3 | Slow network | Throttle connection (Slow 3G) | Loading states shown, no timeouts crashing UI | | |
| 12.2.4 | Intermittent connection | Toggle network on/off | UI recovers when connection restored | | |

### 12.3 Data Edge Cases

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 12.3.1 | No pairs available | Factory with 0 pairs | Empty pair list, appropriate message | | |
| 12.3.2 | Single pair | Factory with 1 pair | Pair shown, no selection issues | | |
| 12.3.3 | Many pairs (200+) | Factory with 200+ pairs | Pagination works (50 per page, up to 200) | | |
| 12.3.4 | Very large numbers | Pool with enormous reserves | Numbers displayed without overflow | | |
| 12.3.5 | Very small numbers | Amounts with many decimals | Formatted correctly, not shown as 0 | | |
| 12.3.6 | Zero reserves pool | Pool with 0/0 reserves | Swap disabled or handled gracefully | | |
| 12.3.7 | Extremely imbalanced pool | Pool with 1:1000000 ratio | UI handles display correctly | | |

---

## 13. INDEXER / BACKEND API

### 13.1 Pairs API

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 13.1.1 | GET /api/v1/pairs | Query endpoint | All pairs returned with asset info | | |
| 13.1.2 | GET /api/v1/pairs/:addr | Query with valid pair address | Single pair returned | | |
| 13.1.3 | GET /api/v1/pairs/:addr (invalid) | Query with nonexistent address | 404 or empty response | | |
| 13.1.4 | GET /api/v1/pairs/:addr/candles | Query with interval=1h | OHLCV candles returned | | |
| 13.1.5 | Candles limit | Query with limit=1001 | Capped at 1000 | | |
| 13.1.6 | Candles from/to filter | Query with date range | Only candles in range returned | | |
| 13.1.7 | GET /api/v1/pairs/:addr/trades | Query recent trades | Trades returned with cursor | | |
| 13.1.8 | Trades limit | Query with limit=201 | Capped at 200 | | |
| 13.1.9 | Trades before cursor | Query with before=id | Trades before that ID | | |
| 13.1.10 | GET /api/v1/pairs/:addr/stats | Query 24h stats | Volume, trades, OHLC, price change returned | | |

### 13.2 Tokens API

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 13.2.1 | GET /api/v1/tokens | Query all tokens | Token list returned | | |
| 13.2.2 | GET /api/v1/tokens/:addr | Query specific token | Token details + volume stats (24h/7d/30d) | | |
| 13.2.3 | GET /api/v1/tokens/:addr/pairs | Query token pairs | Pairs containing token returned | | |
| 13.2.4 | Invalid token address | Query nonexistent token | 404 or empty | | |

### 13.3 Traders API

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 13.3.1 | GET /api/v1/traders/leaderboard | Default query | Traders sorted by total_volume DESC | | |
| 13.3.2 | Leaderboard sort options | Query each sort option | volume, volume_24h, volume_7d, volume_30d, total_trades, total_realized_pnl, best_trade_pnl, worst_trade_pnl, total_fees_paid | | |
| 13.3.3 | Leaderboard limit | Query with limit=201 | Capped at 200 | | |
| 13.3.4 | GET /api/v1/traders/:addr | Query trader profile | Profile with stats returned | | |
| 13.3.5 | GET /api/v1/traders/:addr/trades | Query trader trades | Trade history with cursor | | |
| 13.3.6 | GET /api/v1/traders/:addr/positions | Query positions | All positions per pair | | |
| 13.3.7 | Nonexistent trader | Query address that never traded | 404 or "not found" | | |

### 13.4 Overview API

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 13.4.1 | GET /api/v1/overview | Query global stats | 24h volume, 24h trades, pair count, token count | | |
| 13.4.2 | Overview accuracy | Compare with individual pair stats | Totals match sum of pair stats | | |

### 13.5 CoinGecko API

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 13.5.1 | GET /cg/pairs | Query CG pairs | Pairs in CoinGecko format | | |
| 13.5.2 | GET /cg/tickers | Query tickers | Last price, volume, bid/ask | | |
| 13.5.3 | GET /cg/orderbook | Query with ticker_id and depth | Simulated orderbook (constant product) | | |
| 13.5.4 | GET /cg/historical_trades | Query with ticker_id | Historical trades returned | | |

### 13.6 CoinMarketCap API

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 13.6.1 | GET /cmc/summary | Query CMC summary | All pair summaries | | |
| 13.6.2 | GET /cmc/assets | Query CMC assets | Assets in CMC format | | |
| 13.6.3 | GET /cmc/ticker | Query CMC ticker | Ticker data | | |
| 13.6.4 | GET /cmc/orderbook/:pair | Query with market pair | Simulated orderbook | | |
| 13.6.5 | GET /cmc/trades/:pair | Query trades for pair | Recent trades | | |

### 13.7 Docs

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 13.7.1 | Swagger UI | Navigate to /swagger-ui | Interactive docs rendered | | |
| 13.7.2 | OpenAPI spec | GET /api-docs/openapi.json | Valid OpenAPI JSON returned | | |

---

## 14. INDEXER DATA INTEGRITY

### 14.1 Swap Indexing

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 14.1.1 | Swap event captured | Execute swap on-chain → Check DB | swap_events row created with correct data | | |
| 14.1.2 | Duplicate prevention | Same tx processed twice | Only one swap_events row (trade_exists check) | | |
| 14.1.3 | Unknown pair discovery | Swap on pair not in DB | Pair auto-discovered via pair contract query | | |
| 14.1.4 | Block continuity | Check indexer_state.last_indexed_height | No gaps in indexed blocks | | |
| 14.1.5 | Empty block handling | Block with no swap txs | last_indexed_height advances, no errors | | |
| 14.1.6 | Offer/ask asset resolution | Check swap_events | offer_asset_id and ask_asset_id correctly resolved | | |
| 14.1.7 | Price calculation | Check swap_events.price | price = return_amount / offer_amount | | |
| 14.1.8 | Sender/receiver | Check swap_events | sender and receiver match wasm event attributes | | |

### 14.2 Candle Building

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 14.2.1 | Candle creation on first trade | First trade in interval | New candle created with O=H=L=C=price | | |
| 14.2.2 | Candle update on subsequent trades | Multiple trades in same interval | High/Low updated, Close=latest, volume accumulated | | |
| 14.2.3 | All intervals built | Check candles table | 1m, 5m, 15m, 1h, 4h, 1d candles all present | | |
| 14.2.4 | Interval alignment | Check open_time | Aligned to interval boundaries (e.g., 1h → :00:00) | | |
| 14.2.5 | Volume accuracy | Sum candle volumes for period | Matches sum of swap amounts | | |
| 14.2.6 | Trade count accuracy | Check candle.trade_count | Matches number of swaps in that interval | | |

### 14.3 Trader Tracking

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 14.3.1 | Trader created on first swap | New address swaps | traders row created | | |
| 14.3.2 | total_trades increments | Multiple swaps by same address | total_trades accurate | | |
| 14.3.3 | total_volume accumulates | Multiple swaps | Sum of all offer_amounts | | |
| 14.3.4 | Rolling volumes | Wait 5+ min after trades | volume_24h, volume_7d, volume_30d refreshed | | |
| 14.3.5 | last_trade_at updates | New trade after pause | Timestamp updates | | |

### 14.4 Position & P&L Tracking

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 14.4.1 | Buy position update | Buy quote asset (offer base, receive quote) | net_position_quote increases, avg_entry updated | | |
| 14.4.2 | Sell position update | Sell quote asset (offer quote, receive base) | net_position_quote decreases, P&L realized | | |
| 14.4.3 | Realized P&L calculation | Buy then sell at different price | P&L = (exit_price - avg_entry) × amount | | |
| 14.4.4 | Best trade tracked | Multiple trades with varying P&L | best_trade_pnl = max P&L trade | | |
| 14.4.5 | Worst trade tracked | Multiple trades with varying P&L | worst_trade_pnl = min P&L trade | | |
| 14.4.6 | Fees tracked | Multiple swaps | total_fees_paid = sum of spread + commission | | |
| 14.4.7 | Multi-pair positions | Trade on multiple pairs | Separate position per pair | | |

### 14.5 Tier Sync

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 14.5.1 | Tier sync runs | Wait 10+ minutes | Tier sync loop executes | | |
| 14.5.2 | Registered trader synced | Register on-chain → Wait for sync | traders.tier_id and tier_name updated | | |
| 14.5.3 | Deregistered trader synced | Deregister → Wait for sync | tier_id/tier_name cleared | | |

---

## 15. PERFORMANCE & RELIABILITY

### 15.1 Frontend Performance

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 15.1.1 | Initial page load time | Navigate to / → Measure | Page interactive within 3s | | |
| 15.1.2 | Chart render time | Load chart with 1000+ candles | Renders within 2s | | |
| 15.1.3 | Pair list loading | Open pair dropdown with many pairs | No freeze or lag | | |
| 15.1.4 | Simulation response time | Enter amount → Measure simulation | Response within 2s | | |
| 15.1.5 | Memory usage over time | Use app for 30+ minutes | No memory leak (check DevTools) | | |
| 15.1.6 | Bundle size | Check build output | Reasonable bundle size with lazy loading | | |

### 15.2 API Performance

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 15.2.1 | API response time | Query each endpoint → Measure | < 500ms for all endpoints | | |
| 15.2.2 | Rate limiting | Send > 60 req/s | Rate limited (429 or throttled) | | |
| 15.2.3 | Concurrent requests | 100 simultaneous requests | Server handles without crashing | | |
| 15.2.4 | Large result sets | Query max limits | Response completes without timeout | | |

### 15.3 Indexer Reliability

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 15.3.1 | Indexer restart recovery | Stop and restart indexer | Resumes from last_indexed_height | | |
| 15.3.2 | LCD failover | Primary LCD fails | Falls back to secondary LCD URLs | | |
| 15.3.3 | LCD cooldown | LCD returns errors | 30s cooldown before retry | | |
| 15.3.4 | Block fetch retry | Single block fetch fails | 2s sleep then retry from same height | | |
| 15.3.5 | Database connection loss | Kill DB connection → Restore | Indexer recovers | | |
| 15.3.6 | Poll interval | Measure polling frequency | ~6000ms (POLL_INTERVAL_MS) | | |

---

## 16. SECURITY

### 16.1 Input Validation

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 16.1.1 | XSS in search | Enter `<script>alert(1)</script>` in trader search | Input sanitized, no script execution | | |
| 16.1.2 | SQL injection in API | Send `'; DROP TABLE--` in query params | No SQL injection (parameterized queries) | | |
| 16.1.3 | Oversized input | Enter very long string in any input | Truncated or rejected | | |
| 16.1.4 | CORS enforcement | Request API from unauthorized origin | CORS blocks the request | | |

### 16.2 Transaction Security

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 16.2.1 | Tx params verification | Inspect tx before signing | belief_price, max_spread, amounts match UI | | |
| 16.2.2 | Allowance only for needed amount | Check increase_allowance msg | Allowance matches exact amount needed | | |
| 16.2.3 | Allowance rollback | provide_liquidity fails | decrease_allowance restores previous state | | |

---

## 17. CROSS-BROWSER COMPATIBILITY

| # | Test Case | Browser | Status | Notes |
|---|-----------|---------|--------|-------|
| 17.1 | Full functionality | Chrome (latest) | | |
| 17.2 | Full functionality | Firefox (latest) | | |
| 17.3 | Full functionality | Safari (latest) | | |
| 17.4 | Full functionality | Edge (latest) | | |
| 17.5 | Full functionality | Brave (latest) | | |
| 17.6 | Wallet extensions | Chrome + all wallets | | |
| 17.7 | Wallet extensions | Firefox + all wallets | | |
| 17.8 | Wallet extensions | Brave + all wallets | | |

---

## 18. CONFIGURATION & ENVIRONMENT

### 18.1 Frontend Environment

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 18.1.1 | Local network config | VITE_NETWORK=local | Uses localterra, localhost LCD/RPC | | |
| 18.1.2 | Testnet config | VITE_NETWORK=testnet | Uses rebel-2, testnet endpoints | | |
| 18.1.3 | Mainnet config | VITE_NETWORK=mainnet | Uses columbus-5, mainnet endpoints | | |
| 18.1.4 | Factory address | Check VITE_FACTORY_ADDRESS | Points to correct factory contract | | |
| 18.1.5 | Fee discount address | Check VITE_FEE_DISCOUNT_ADDRESS | Points to correct fee discount contract | | |
| 18.1.6 | CL8Y token address | Check VITE_CL8Y_TOKEN_ADDRESS | Points to correct CL8Y token | | |
| 18.1.7 | LCD URL | Check VITE_TERRA_LCD_URL | Accessible and responding | | |
| 18.1.8 | RPC URL | Check VITE_TERRA_RPC_URL | Accessible and responding | | |
| 18.1.9 | Indexer URL | Check VITE_INDEXER_URL | Accessible and responding | | |
| 18.1.10 | WalletConnect project ID | Check VITE_WC_PROJECT_ID | Valid WC project ID | | |

### 18.2 Indexer Configuration

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 18.2.1 | DATABASE_URL | Check connection | PostgreSQL reachable | | |
| 18.2.2 | FACTORY_ADDRESS | Check value | Correct factory contract | | |
| 18.2.3 | CORS_ORIGINS | Check value | Frontend origin(s) allowed | | |
| 18.2.4 | LCD_URLS | Check endpoints | All LCD URLs reachable | | |
| 18.2.5 | FEE_DISCOUNT_ADDRESS | Check value | Correct contract | | |
| 18.2.6 | API_PORT | Check binding | API accessible on configured port | | |
| 18.2.7 | POLL_INTERVAL_MS | Check polling | Polling at configured interval | | |
| 18.2.8 | LCD_TIMEOUT_MS | Check timeouts | 8s timeout applied | | |
| 18.2.9 | RATE_LIMIT_RPS | Test rate limiting | 60 req/s limit applied | | |

---

## 19. DATABASE INTEGRITY

| # | Test Case | Steps | Expected Result | Status | Notes |
|---|-----------|-------|-----------------|--------|-------|
| 19.1 | Migrations applied | Check migration status | All 3 migrations applied | | |
| 19.2 | Foreign keys valid | Check pairs → assets FK | All pair asset IDs exist in assets table | | |
| 19.3 | No orphaned swaps | Check swap_events → pairs FK | All swap pair_ids exist | | |
| 19.4 | No orphaned positions | Check trader_positions → pairs | All position pair_ids exist | | |
| 19.5 | Candle pair consistency | Check candles → pairs | All candle pair_ids exist | | |
| 19.6 | Unique constraints | Check duplicates | No duplicate swap events (tx_hash + pair_id) | | |
| 19.7 | Indexer state | Check indexer_state table | last_indexed_height and indexer_version present | | |

---

## 20. KNOWN LIMITATIONS TO VERIFY

| # | Item | Expected Behavior | Status | Notes |
|---|------|-------------------|--------|-------|
| 20.1 | Reverse simulation unused | "You Receive" input not editable (no reverse sim) | | |
| 20.2 | Router unused | No multi-hop routing (ROUTER_CONTRACT_ADDRESS defined but unused) | | |
| 20.3 | Native token swaps | UI is CW20-focused; native token swap flow not wired | | |
| 20.4 | Liquidity events not indexed | liquidity_events table exists but parser only captures swaps | | |
| 20.5 | 1w candles | API accepts interval=1w but indexer doesn't build weekly candles | | |
| 20.6 | first_trade_at | May remain NULL in traders table (not set in upsert) | | |
| 20.7 | token unique_traders | token_volume_stats.unique_traders never updated | | |
| 20.8 | Volume calculation | Global stats sums offer_amount only (not USD-denominated) | | |

---

## SIGN-OFF

| Role | Name | Date | Signature |
|------|------|------|-----------|
| QA Tester | | | |
| Dev Lead | | | |
| Product Owner | | | |

### Summary

| Category | Total Tests | Pass | Fail | Blocked | Skipped |
|----------|------------|------|------|---------|---------|
| 1. Wallet Connection | 67 | | | | |
| 2. Swap Functionality | 34 | | | | |
| 3. Liquidity Pools | 22 | | | | |
| 4. Fee Tiers & Discounts | 18 | | | | |
| 5. Charts & Analytics | 31 | | | | |
| 6. Trader Profile | 17 | | | | |
| 7. Create Pair | 11 | | | | |
| 8. Token Display | 9 | | | | |
| 9. Navigation & Routing | 16 | | | | |
| 10. Theme & UI/UX | 15 | | | | |
| 11. Responsive/Mobile | 15 | | | | |
| 12. Error Handling | 10 | | | | |
| 13. API Endpoints | 22 | | | | |
| 14. Indexer Integrity | 23 | | | | |
| 15. Performance | 12 | | | | |
| 16. Security | 6 | | | | |
| 17. Cross-Browser | 8 | | | | |
| 18. Configuration | 19 | | | | |
| 19. Database Integrity | 7 | | | | |
| 20. Known Limitations | 8 | | | | |
| **TOTAL** | **370** | | | | |
