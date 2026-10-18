# Asset accounting

**Later movements:** [BNB_FOLLOWUP.md](BNB_FOLLOWUP.md) carries exact post-snapshot BNB/USDT/ETH equations, gas and attribution limits. The $3907.19 figure below remains the original dated valuation; subsequent conversions and outside funding are not counted as new theft.

All quantities below use exact integer base units or decimal arithmetic. Dates are UTC. The positions are captured historical holdings, not a continuing balance monitor. An economic unit can appear repeatedly as a transfer edge, a router hop, and a burn/unlock representation; those appearances are not independent losses.

## CL8Y

Nine source bridge deposits are in scope: nonce 150 and nonces 152–159. One is vFDUSD and two are USDT; six deposits carry CL8Y, including a one-wei test. Eight corresponding BSC withdrawals executed; the vFDUSD deposit had no submitted destination withdrawal at the checked hash.

| CL8Y component | Amount |
| --- | ---: |
| Gross debited on Terra | 32148.950800756371923197 |
| Source bridge fees | 96.446852402269115768 |
| Net burned / released on BSC | 32052.503948354102807429 |
| CL8Y debited in 20 successful sales | 6600 |
| Of that sale debit: routing fee | 16.5 |
| Of that sale debit: trading-route input | 6583.5 |
| CL8Y remaining at `[HACKER_A]` | 25452.503948354102807429 |

`32148.950800756371923197 − 96.446852402269115768 = 32052.503948354102807429`

`32052.503948354102807429 − 6600 = 25452.503948354102807429`

The 1646.257444506482572970 CL8Y initially observed at 04:00 is now traced: earlier nonce 150 released 8246.257444506482572970 CL8Y and 6600 CL8Y was sold before 04:00. The victim stated those earlier actions were unauthorized. This supersedes the first investigation's unresolved-opening-inventory description.

## BNB

| Native component | BNB |
| --- | ---: |
| Opening balance at the expanded start | 0 |
| Three independently verified external funding credits | 0.000062918448329107 |
| Actual native credits from the 20 sale call trees | 3.943439176513535698 |
| Gas across all 56 outgoing transactions | 0.00620250361862 |
| Onward native principal sent | 0 |
| Captured final BNB balance | 3.937299591343244805 |
| Sales less all 56 gas costs, excluding outside funding | 3.937236672894915698 |

`0 + 0.000062918448329107 + 3.943439176513535698 − 0.00620250361862 = 3.937299591343244805`

All 56 account nonces, 0–55, are captured: eight withdrawal submissions, 22 approvals, 20 successful sales and six failed swaps. Every outer transaction has zero native `value`. Actual receipt gas is `gasUsed × effectiveGasPrice`, not the maximum gas allowance. All 55 unique own-transaction-block balance checks close without unexplained residuals.

WBNB unwrap events sum to 3.937552621420712913 WBNB inside the routes. Actual native credits to the recipient sum to 3.943439176513535698 BNB. The native call trees, excluding reverted branches and DELEGATECALL apparent values, establish the payout measure. Wrapped transit is not an additional wallet gain. No WBNB remained at the checked recipient snapshot.

The three funding source controllers are un-attributed. Their 0.000062918448329107 BNB is shown separately rather than counted as victim-derived sale proceeds.

## Stablecoins

| Asset | Gross source debit | Bridge fee | Net |
| --- | ---: | ---: | ---: |
| USDT | 580.076585225842215326 | 1.740229755677526645 | 578.336355470164688681 |
| vFDUSD | 10.184892 | 0.030554 | 10.154338 |

The USDT releases reached `[HACKER_A]` and remained there at the captured checks. The vFDUSD amount is an unreleased bridge claim, not a received destination balance. An all-zero withdrawal getter and unused source nonce establish absence at the checked destination hash; they do not establish cancellation or extinguishment of the claim.

## Separately dated bridge recovery — 16 October 2026

The owner-authorized BSC vault payout recovered **10.154338 vFDUSD net** to `0x21ddb4bcb5f00430cb39d5fea2d382bbeaa076ba`: **1015433800 base units at eight BSC decimals**, equal to the original **10154338 base units at six Terra decimals**. It did not reimburse the **0.030554** source fee. This later response changes the recovery status of that net principal, not the historical source debit/fee/net figures or dated USD valuation. It is not a second attacker receipt or ordinary execution of the original withdrawal. Nonce 153 remains unused, with payment to the attacker blocked by the installed bridge blacklist.

The six response transactions have separate operator gas accounting in [ledgers/bridge-response-transactions.csv](ledgers/bridge-response-transactions.csv), outside the original 56-transaction attacker gas total. See [BRIDGE_RECOVERY.md](BRIDGE_RECOVERY.md) for receipts, deployment scope, unpause and control dependencies.

## Separately dated compensation and NFT recovery — 17–18 October 2026

The [recovery follow-up](RECOVERY_UPDATE_20261018.md) verifies a **25,452.503948354102807429 CL8Y** transfer (`25452503948354102807429` base units at 18 decimals) from the project reserve to the designated BSC recovery wallet. This compensates the matching attacker-held amount using existing reserve inventory; it is not a mint, seizure or reversal of the original theft. The attacker still held that CL8Y amount and was sender-blacklisted at block 128324235. Reserve transfer gas was **0.00000720522 BNB**, paid by the reserve wallet and excluded from the original attacker gas total.

At that snapshot the recovery wallet held the CL8Y compensation and **10.154338 vFDUSD** already recovered on 16 October. Count the vFDUSD recovery once. The custodian reported holding both pending a final destination; receipt in this address is not final delivery to the victim. The original theft flow totals, fees and dated USD valuation are not rewritten or increased by reserve compensation.

Penguin #1509's completed transfer and recipient ownership are verified. No NFT USD valuation or aggregate recovered USD total is established by this update.

## Native Terra / Osmosis route

| Movement | Gross | Tax / explicit route deduction | Recipient / next-leg amount |
| --- | ---: | ---: | ---: |
| `[VICTIM_A]` USTC transfer | 44700 USTC | 670.5 USTC | 44029.5 USTC to `[HACKER_C]` |
| `[VICTIM_A]` LUNC transfer | 1270000 LUNC | 19050 LUNC | 1250950 LUNC to `[HACKER_C]` |
| `[VICTIM_B]` LUNC transfer | 5160000 LUNC | 77400 LUNC | 5082600 LUNC to `[HACKER_C]` |
| Osmosis input routing | 44029.5 USTC | 44.0295 USTC | 43985.4705 USTC to pool 562 |
| Pool 562 output | 4353371.354935 LUNC | 22165.299087 + 6999.568133 LUNC affiliate payouts | 4324206.487715 LUNC returned to Terra |

The observed affiliate amounts are floor-rounded 57-bps and 18-bps charges against the memo's minimum-output reference of 3888648.962795 LUNC. They are not percentages of actual pool output. Pool fees/spread are already reflected in the measured output.

`1250950 + 5082600 + 4324206.487715 − 4.284242 relay gas = 10657752.203473 LUNC`

The original 44700 USTC is converted principal; adding it to the returned LUNC would count the same value twice. A separate protorev module backrun in the Osmosis transaction records 66.635398 OSMO profit. It is not attributed to the attacker and is excluded. Osmosis relayer gas of 0.120637 OSMO is also not a debit from the tracked victim/attacker wallets.

## Dated USD spot valuation

This is the value of **net traced non-CL8Y proceeds**, not total victim economic loss, a realized USD profit, acquisition basis or a fiat off-ramp receipt. The amounts come from the captured incident positions; the prices are separately captured USD spot quotes on 2026-10-15, around 10:24 UTC.

| Asset | Quantity valued | USD price | USD value, rounded |
| --- | ---: | ---: | ---: |
| USDT | 578.336355470164688681 | 0.999077 | 577.80 |
| BNB, gas deducted and external funding excluded | 3.937236672894915698 | 719.04 | 2831.03 |
| LUNC, including converted USTC | 10657752.203473 | 0.00004676 | 498.36 |
| **Total** | | | **3907.19** |

The unrounded result is 3907.189701326823390163267437 USD. The excluded items are locked CL8Y, the unreleased vFDUSD claim, unverified unsolicited tokens, and outside BNB gas funding. Source bridge/routing fees and taxes are not attacker terminal inventory. The calculation already deducts the observed BSC gas cost and the relay gas cost.

Quote observations: BNB timestamp 10:24:17 UTC; LUNC/USDT timestamps 10:24:35 UTC. Sources and exact arithmetic are captured in [evidence/valuation/](evidence/valuation/). This is not an execution-time historical quote. The captured spot prices can differ from prices at the earlier swaps or at any later recovery.
