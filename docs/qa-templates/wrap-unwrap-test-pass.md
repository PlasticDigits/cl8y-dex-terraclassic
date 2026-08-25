## QA Test Pass — Wrap/Unwrap Flows

**Date:** <!-- YYYY-MM-DD -->
**Tester:** <!-- @gitlab-handle -->
**Environment:** <!-- testnet / mainnet / local -->
**Browser:** <!-- Chrome 130 / Safari 18 / etc. -->
**Wallet:** <!-- Keplr Extension v0.x (LocalTerra) / Station Extension v4.x (columbus-5 only — not LocalTerra, see #235) -->

---

### Prerequisites
- [ ] Treasury and wrap-mapper contracts deployed
- [ ] cLUNC and cUSTC CW20 tokens created with wrap-mapper as minter
- [ ] Denom mappings registered (uluna → cLUNC, uusd → cUSTC)
- [ ] Wrappers registered on treasury (uluna → wrap-mapper, uusd → wrap-mapper)
- [ ] Wrap-mapper set on router (`SetWrapMapper`)
- [ ] Treasury funded with native tokens (≥40M USTC in production)
- [ ] Test wallet has native LUNC and USTC balances
- [ ] Query wrap-mapper `Config` — record `fee_wrap_bps` / `fee_unwrap_bps` (or pre-migrate single `fee_bps`). Post-migrate columbus-5 target **200 / 51** ([#516](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/516)); LocalTerra deploy default often **50** unless changed. Do not hardcode.

### Mainnet Coolify env checklist (#507)

Before testing on `https://dex.cl8y.com`, confirm Coolify frontend build-args (rebuild required after change). Template: [`deployments/mainnet-soft-launch/wrap-enablement.env.example`](../../deployments/mainnet-soft-launch/wrap-enablement.env.example). Playbook: [`skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md`](../../skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md).

| Key | Expected mainnet value |
|-----|------------------------|
| `VITE_WRAP_MAPPER_ADDRESS` | `terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2` |
| `VITE_TREASURY_ADDRESS` | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` (CMM treasury — **not** governance multisig `terra1zlmv2…`) |
| `VITE_LUNC_C_TOKEN_ADDRESS` | `terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg` |
| `VITE_USTC_C_TOKEN_ADDRESS` | `terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch` |

- [ ] All four keys set on Coolify and frontend image rebuilt
- [ ] Token selector shows **cLUNC** / **cUSTC** (not legacy LUNC-C / USTC-C labels)
- [ ] Soft-launch `frontend.env.example` from deploy script has wrap keys **commented only** (not active)

### Fee formula (wrap-mapper split fees, #516)

For amount `A` and the **matching** on-chain fee (`fee_wrap_bps` on wrap, `fee_unwrap_bps` on unwrap; transitional `fee_bps` only when both split fields are absent):

`net = A − floor(A × bps / 10_000)`

When that fee > 0, UI must **not** claim 1:1. Do not claim “2% flat” on unwrap (tax still applies).

| Direction | Expected You Receive |
|-----------|----------------------|
| **Wrap** | `net` per **wrap** fee only (`MsgExecuteContract` untaxed). 10 000 @ 200 → **9 800** |
| **Unwrap** | `net` after **unwrap** fee, then Classic burn tax (`floor(net × burn_tax_rate)`). 10 000 @ 51 + 1.5% → **9 800** (not 9 653). Pre-migrate @ 200 + 1.5% → **9 653** — quote that until config is live. |

Playbooks: [`skills/AGENTS_WRAP_MAPPER_SPLIT_FEES.md`](../../skills/AGENTS_WRAP_MAPPER_SPLIT_FEES.md) (**W12–W15**), [`skills/AGENTS_WRAP_UNWRAP_BURN_TAX.md`](../../skills/AGENTS_WRAP_UNWRAP_BURN_TAX.md) (**W8–W11**), [`skills/AGENTS_NATIVE_WRAP_TAX.md`](../../skills/AGENTS_NATIVE_WRAP_TAX.md).

### 0. Discoverability (post–Coolify wrap env)

- [ ] **More → Wrap** opens `/wrap` (label **Wrap**, not Mint / UST1)
- [ ] Swap token search lists **LUNC**, **cLUNC**, **USTC**, **cUSTC** even when no wrap factory pairs exist (visible text is never `uluna` / `uusd`; ids stay those denoms — [#630](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/630))
- [ ] Dedicated `/wrap` page: Wrap/Unwrap tabs + LUNC/cLUNC and USTC/cUSTC asset toggles

### 1. Direct Wrap (Native → Wrapped CW20)

#### LUNC → cLUNC
- [ ] Select LUNC as "From" and cLUNC as "To" on swap page (or use **More → Wrap**)
- [ ] Inline note references wrap fee (not "1:1" when wrap fee > 0)
- [ ] Button label is **Wrap** (not "Swap") on the direct wrap route
- [ ] Enter amount — estimated output = `net` per fee formula above
- [ ] Execute wrap — transaction succeeds
- [ ] LUNC balance decreases by entered amount
- [ ] cLUNC balance increases by **net** (not gross when fee > 0)
- [ ] Treasury LUNC balance increases by entered amount

#### USTC → cUSTC
- [ ] Select USTC as "From" and cUSTC as "To" on swap page
- [ ] Inline note references wrap fee (not "1:1" when wrap fee > 0)
- [ ] Button label is **Wrap**
- [ ] Execute wrap — transaction succeeds
- [ ] USTC balance decreases by entered amount
- [ ] cUSTC balance increases by **net**
- [ ] Treasury USTC balance increases by entered amount

### 2. Direct Unwrap (Wrapped CW20 → Native)

#### cLUNC → LUNC
- [ ] Select cLUNC as "From" and LUNC as "To" on swap page
- [ ] Inline note references unwrap fee **and** burn tax on payout (not "1:1" when unwrap fee > 0; not “2% flat”)
- [ ] Exchange-deposit warning visible (withdraw to own wallet first) — [#512](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/512)
- [ ] Button label is **Unwrap**
- [ ] Enter amount — estimated output = post-unwrap-fee then burn tax (post-migrate 10 000 @ 51 + 1.5% → **≈9 800**; pre-migrate @ 200 + 1.5% → **9 653**) — [#516](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/516)
- [ ] Execute unwrap — transaction succeeds
- [ ] cLUNC balance decreases by entered amount
- [ ] LUNC balance increases by **post-tax** native received (matches quote)
- [ ] Treasury LUNC balance decreases by unwrap gross

#### cUSTC → USTC
- [ ] Select cUSTC as "From" and USTC as "To" on swap page
- [ ] Execute swap — transaction succeeds
- [ ] cUSTC balance decreases by entered amount
- [ ] USTC balance increases by net native received
- [ ] Treasury USTC balance decreases by unwrap gross

### 3. Native Input Swap (Native → CW20 via Wrap + Router)

- [ ] Select LUNC as "From" and a non-wrapped CW20 (e.g. EMBER) as "To"
- [ ] Route display shows wrap step (LUNC → cLUNC → … → EMBER)
- [ ] Inline note: "This swap will wrap your tokens"
- [ ] Enter amount — estimated output updates correctly (wrap fee netted on CW20 leg)
- [ ] Execute swap — single transaction with multiple messages succeeds
- [ ] LUNC balance decreases
- [ ] CW20 output token balance increases
- [ ] Treasury LUNC balance increases by input amount

### 4. Native Output Swap (CW20 → Native via Router + Unwrap)

- [ ] Select a CW20 token (e.g. EMBER) as "From" and LUNC as "To"
- [ ] Route display shows unwrap step (EMBER → … → cLUNC → LUNC)
- [ ] Inline note: "This swap will unwrap your tokens"
- [ ] Execute swap — transaction succeeds with `unwrap_output: true`
- [ ] Quoted receive nets mapper **`fee_unwrap_bps`** on final unwrap leg
- [ ] CW20 input token balance decreases
- [ ] LUNC balance increases
- [ ] Treasury LUNC balance decreases by output amount

### 5. Native-to-Native Swap (Wrap + Router + Unwrap)

- [ ] Select LUNC as "From" and USTC as "To"
- [ ] Route display shows both wrap and unwrap steps
- [ ] Inline note: "This swap will wrap and unwrap your tokens"
- [ ] Execute swap — multi-message transaction succeeds
- [ ] LUNC balance decreases
- [ ] USTC balance increases (net of both mapper fees and taxes)
- [ ] Treasury LUNC balance increases, Treasury USTC balance decreases

### 6. Treasury Balance Integrity

- [ ] Record treasury LUNC and USTC balances before a sequence of wrap/unwrap operations
- [ ] Perform: 3 wraps, 2 unwraps, 1 native input swap, 1 native output swap
- [ ] After all operations: treasury native balance ≥ total CW20 supply for each denom
- [ ] Query CW20 token_info for cLUNC and cUSTC — `total_supply` matches expected minted minus burned
- [ ] No "phantom" tokens: every cLUNC in circulation is backed by LUNC in treasury

### 7. Rate Limits

- [ ] If rate limit is configured for uluna: attempt to wrap more than `max_amount_per_window`
- [ ] Transaction fails with a clear rate-limit error message
- [ ] Wait for the rate limit window to expire, retry — succeeds
- [ ] If rate limit is configured for uusd: same checks as above
- [ ] Rate limit state resets correctly after window expiry
- [ ] Rate limits apply per-denom independently (wrapping LUNC does not affect USTC limit)

### 8. Paused State

- [ ] If wrap-mapper is paused by governance: attempt to wrap → clear "paused" error / CTA **Wrapping is Temporarily Paused** (SEC-A02)
- [ ] If wrap-mapper is paused: attempt to unwrap → clear "paused" error
- [ ] Existing CW20 swaps (not involving wrap/unwrap) still work while wrap-mapper is paused
- [ ] Pause CTA precedence over rate-limit CTA when both would apply ([`skills/AGENTS_FRONTEND_SWAP_SAFETY_CTA.md`](../../skills/AGENTS_FRONTEND_SWAP_SAFETY_CTA.md))

### 8b. Columbus-5 wrap pause smoke (#503)

Operator rehearsal on mainnet (off-peak) or documented dry-run. Playbook: [`docs/runbooks/wrap-mapper-pause.md`](../runbooks/wrap-mapper-pause.md). LocalTerra substitute: `make smoke-wrap-mapper-pause`.

- [ ] Preflight: `./scripts/check-ust1-wrap-ops-health.sh` (wrap-mapper not paused)
- [ ] `set_paused: true` tx hash recorded
- [ ] Wrap rejected + unwrap rejected while paused
- [ ] UI pause CTA confirmed on `dex.cl8y.com`
- [ ] `set_paused: false` tx hash recorded; tiny wrap+unwrap succeed
- [ ] Evidence commented on [GitLab #503](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/503) (or dry-run + LocalTerra transcript if keys unavailable)

### 9. Error Handling & Edge Cases

- [ ] Wrap with zero amount → blocked or clear error before submission
- [ ] Unwrap with zero amount → blocked or clear error
- [ ] Wrap an unsupported denom (not uluna/uusd) → clear error
- [ ] Unwrap a non-registered CW20 (not cLUNC/cUSTC) → clear error
- [ ] Unwrap more than treasury holds → transaction reverts with clear error
- [ ] Swap with `unwrap_output: true` but no wrap-mapper set on router → clear error
- [ ] Insufficient native balance for wrap → wallet blocks or clear error
- [ ] Insufficient CW20 balance for unwrap → wallet blocks or clear error
- [ ] Slippage tolerance enforced on native swaps (`minimum_receive` on post-unwrap net; see [`skills/AGENTS_ROUTER_MINIMUM_RECEIVE.md`](../skills/AGENTS_ROUTER_MINIMUM_RECEIVE.md))
- [ ] Deadline enforcement still works on native swaps

### 10. Pool UI — Native Token Liquidity

#### Provide Liquidity
- [ ] For a pair containing cLUNC or cUSTC: "Use native (auto-wrap)" checkbox appears
- [ ] Checking the box: provide liquidity using native LUNC/USTC (auto-wraps in same TX)
- [ ] Transaction succeeds — LP tokens received
- [ ] Treasury balance updated correctly from the wrap

#### Withdraw Liquidity
- [ ] For a pair containing cLUNC or cUSTC: "Receive as wrapped tokens" checkbox appears
- [ ] With checkbox checked: receive cLUNC/cUSTC on withdrawal
- [ ] With checkbox unchecked: receive native LUNC/USTC (auto-unwrap)
- [ ] Treasury balance updated correctly from any unwrap

### 11. Token Selector UI

- [ ] Native LUNC appears in the "From" token dropdown
- [ ] Native USTC appears in the "From" token dropdown
- [ ] Native LUNC appears in the "To" token dropdown
- [ ] Native USTC appears in the "To" token dropdown
- [ ] cLUNC and cUSTC also appear separately (users can choose either)
- [ ] Selecting a native token shows the correct balance (bank query, not CW20)
- [ ] Swap direction toggle (↕) works correctly with native tokens

### 12. Cross-browser / Responsive

- [ ] All wrap/unwrap flows work on Desktop Chrome
- [ ] All wrap/unwrap flows work on Desktop Firefox
- [ ] Wrap/unwrap note text renders correctly on mobile viewports
- [ ] Pool native checkboxes are tappable on mobile

### Notes
<!-- Additional observations, edge cases found, performance issues, etc. -->
