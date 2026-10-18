> Archive 20261016_001. This is the factual publication derivative; response recommendations and proposed fixes are excluded. Original chain events occurred 2026-10-15.

# Wallet incident review — 15 October 2026

**17–18 October recovery update:** [RECOVERY_UPDATE_20261018.md](RECOVERY_UPDATE_20261018.md) verifies the project reserve's **25,452.503948354102807429 CL8Y** compensation to the designated BSC recovery wallet and Penguin #1509's completed transfer to the Terra recovery recipient. Police reporting is participant-reported; the private conversation is anonymized and paraphrased. Original theft accounting remains historical.

**Exchange outreach update:** The requester confirmed sending both prepared FixedFloat and KuCoin emails from `contact@cl8y.com`. See [COMMUNICATIONS.md](COMMUNICATIONS.md). Delivery, exchange acknowledgement and any resulting asset recovery remain unverified; the BSC on-chain notice has no captured broadcast receipt.

**16 October response update:** [BRIDGE_RECOVERY.md](BRIDGE_RECOVERY.md) records the completed **10.154338 vFDUSD** vault payout to the newly designated BSC recovery wallet, blacklist installation for both BSC attacker wallets, and subsequent BSC unpause. The original nonce 153 remains unused; payment is blocked by the installed recipient blacklist. Historical theft-accounting amounts and cutoff below remain dated to 15 October.

**Subsequent-flow update:** See [BNB_FOLLOWUP.md](BNB_FOLLOWUP.md). Later receipts show the original USDT sold into BNB, 2.81 BNB bridged into ETH, and 2.54 BNB forwarded less gas to a FixedFloat-attributed address. The original balances and no-onward-payment finding below apply only to their stated pre-09:44 UTC snapshot; they are not current balances.

**Corrected finding:** The expanded trace confirms an earlier unauthorized deposit from `[VICTIM_RECOVERY]` and **20 completed CL8Y sales into BNB**. The initial BSC review started at 04:00 UTC and missed this earlier wave. Its incident-wide “no cash-out” implication is superseded by this report. The victim confirmed that the earlier deposit and sales were unauthorized.

No exploit or takeover of our reviewed contracts was identified. These transfers use valid wallet signing authority and normal funded contract paths. The mechanism behind the unauthorized signatures remains unresolved; this is a scoped incident assessment, not a general certification of every contract or host.

**Identity handling:** Personal names and handles are excluded. Wallet addresses and transactions are included under pseudonymous labels. `[HACKER_A]`, `[HACKER_B]`, and `[HACKER_C]` identify reported attacker-associated addresses, not verified real-world identities or necessarily different people. Pools, routing contracts, fee recipients, relayers, token issuers, and funding senders are not attributed as attackers merely because they handled a transaction.

**Custody, victim-reported:** `[VICTIM_RECOVERY]` is a **Ledger wallet**. `[VICTIM_A]` and `[VICTIM_B]` are **software wallets**. The Ledger-address transaction is verified on-chain, but this does not establish whether the device signed it, a seed or backup was used elsewhere, or another signing path was involved. A Ledger device or firmware compromise has not been established. The “recovery” label does not establish that this address is secure.

## Verified proceeds and terminal holdings

BSC financial coverage: **2026-10-15 00:00 UTC through 2026-10-15 09:12:20 UTC**. This combines the expanded historical log search, all 56 outgoing account transactions, all 20 sale call trees, pinned balance checks, and the final tail scan. Amounts are native/token units, not a USD profit or tax calculation.

| Asset / movement at `[HACKER_A]` | Verified incident amount | Closing captured holding |
| --- | --- | --- |
| CL8Y | 32052.503948354102807429 bridged in; 6600 debited for sales | 25452.503948354102807429 |
| USDT | 578.336355470164688681 bridged in; no verified onward debit | 578.336355470164688681 |
| BNB from sales | 3.943439176513535698 actual native credits from 20 sales | Included in native equation below |
| External BNB funding | 0.000062918448329107 across 3 direct transfers | Separate from sale proceeds |
| BNB gas cost | 0.00620250361862 across all 56 outgoing transactions | Includes 6 failed swaps |
| BNB final balance | Opening balance + funding + sale receipts − gas | 3.937299591343244805 |
| WBNB transit | 3.937552621420712913 unwrapped inside sale routes | 0 direct wallet holding |

The CL8Y balance closes exactly:

`32052.503948354102807429 received − 6600 sold/debited = 25452.503948354102807429 CL8Y held`

The 1646.257444506482572970 CL8Y observed at 04:00 is now explained: it is the residue of the earlier 8246.257444506482572970 CL8Y release after 6600 CL8Y of sales. It is part of the traced unauthorized activity, rather than unresolved opening inventory.

A final balance/nonce spot check at **2026-10-15 09:44:22 UTC** found the same CL8Y, USDT, BNB, and WBNB balances and account nonce 56. The complete flow-log coverage stated above still ends at 09:12:20 UTC; the later check is a balance/nonce confirmation, not an additional complete log scan.

The BNB balance also closes exactly:

`0 opening + 0.000062918448329107 funding + 3.943439176513535698 sales − 0.00620250361862 gas − 0 onward principal = 3.937299591343244805 BNB held`

All **56 account nonces, 0–55**, are captured: **8 bridge submissions, 22 approvals, 20 successful sales, and 6 failed swap attempts**. Every outer transaction sent zero native BNB value. The 55 unique transaction-block balance checks have zero unexplained residual after matching the three funding transfers and the actual sale credits. CL8Y was liquidated into BNB, and the BNB remained at `[HACKER_A]`; no onward BNB principal payment or exchange/off-ramp transfer was found in this covered period.

Actual credits to the wallet determine the proceeds. The **3.943439176513535698 BNB** credited from the 20 sale call trees exceeds the **3.937552621420712913 WBNB** unwrapped in those routes. Several routes settle native funds through their execution contracts. The call ledger records the exact native edges; WBNB transit must not be counted as a second profit. Sale receipts less all 56 transaction gas costs equal **3.937236672894915698 BNB**. The final wallet balance additionally includes the **0.000062918448329107 BNB** of separately verified funding.

The nine source deposits also include **10.154338 vFDUSD net** burned on Terra for `[HACKER_B]`, with no BSC withdrawal record at the checked hash and unused source nonce. This is **unsubmitted, not cancelled**. `[HACKER_B]` held zero queried CL8Y, USDT, WBNB, vFDUSD, and BNB at the BSC snapshot.

## How the sales were routed

The 20 successful sales debited **6600 CL8Y total**, including **16.5 CL8Y** paid to routing-fee recipient `0xcd6b980029e6e6e0733ac8ec3e02be9410d09799`. Those fees are already included in the 6600 CL8Y debit; the remaining **6583.5 CL8Y** entered trading routes.

The calls use routers including `0x111111125421ca6dc452d289314280a0f8842a65` and `0x6131b5fae19ea4f9d964eac0408e4408b66337b5`. Execution contracts and pools include `0x111116058b9828cdcdd810c502b925c14f34ca11`, `0x8f10b468b06c6fd214b65f87778827f7d113f996`, `0x09ad820aac5779683b481c4674208a4e1b024afa`, `0x20f6ee51340adeed01a59b0e65cb3703f3dc860c`, and `0xc10ee9031f2a0b84766a86b55a8d90f357910fb4`. These are observed transaction roles, not findings about who owns those contracts.

Routes include USDT, BUSD, WBNB, LTC, TRX, and a token reporting the symbol “龙虾”. The intermediary tokens move between pools and routers; they are not additional net amounts received by the attacker wallets. The ledger below records **388 ERC20 routing edges** and **128 successful native call edges** from these sales. Separately, 14 unsolicited, unverified incoming token/log credits lack demonstrated economic value and are excluded from proceeds.

The sales and asset conversion are confirmed. A concealment or laundering purpose is not established merely by the use of intermediate tokens or aggregation contracts.

A later transaction at 06:06:59 UTC approves **1000 CL8Y** to `0x84d88856a62ce42165d3f196124d5bd50f17c2bb`. That approval did not itself move funds. It is distinct from the 20 earlier completed sales:

[0xb9086ae2acaf5ce4ac47cece5dae1594d0c99a63cc2f6064774cbf4d9519245f](https://bscscan.com/tx/0xb9086ae2acaf5ce4ac47cece5dae1594d0c99a63cc2f6064774cbf4d9519245f)

## Observed containment

At the 08:54 UTC source/control snapshot, both reported BSC destinations were blocked from sending CL8Y, read-only transfer simulations reverted, and the BSC bridge was paused. The installed withdrawal guard contained a rate-limit module; read-only guard simulations permitted both destinations for USDT/vFDUSD. Those are dated observations, not evidence that the controls remained unchanged later.



`[HACKER_C]` held 10657752.203473 LUNC at the captured relay snapshot. The BSC CL8Y restriction did not extend to native BNB, USDT or Terra bank balances. No production setting was changed by the investigation.

## Contract and authority checks

Captured source transfers use valid ordinary wallet signatures. The bridge receives funded CW20 sends, burns the net representation, and collects the recorded fee. LP withdrawals burn configured LP shares and refund the signer. Unwraps consume wrapped tokens before treasury payouts. No authorization bypass or excess withdrawal was identified in these traces.

The review covered current admins and code histories for core contracts, seven relevant CW20s, the fee registry, and all 22 factory pools. Their code-history entries predate the first identified unauthorized deposit at 02:10:35 UTC. All 22 pools retained expected DEX multisig admins and fee treasury, had no installed hooks, and were unpaused in captured queries.

Core configurations captured before 04:19 matched current reads except the router’s route counter. Expanded 02:10–04:15 event searches showed ordinary swap, wrap, bridge, and oracle activity, with no administrative/configuration actions. This supports the earlier-wave review without treating the 04:00 snapshot as if it preceded that wave.

BSC core owners, implementation pins, bridge operator/canceler lists, guard, and cancellation window matched between 01:59:58 UTC and current queries. All eight executed BSC releases reconcile to matching source deposits. Unrelated bridge nonce 151 has a different source and destination; it is excluded rather than attributed by timing alone.

The affected source accounts do not match captured Terra admin, governance, or minter addresses. None of the affected source keys matches any of the three constituent keys in the 2-of-3 DEX governance multisig. The Ledger address’s derived EVM account matches neither reported BSC destination nor a captured BSC core owner. Checked derived accounts and reported destinations have no membership in the current bridge-guard manager roles or the actual CL8Y guard authority’s required admin/execution/blacklist-clear roles.

Software-wallet authz grants were empty and checked allowances were zero or absent. Those current observations do not establish historical absence or secure a wallet whose signing authority was used without consent.

The cause remains unresolved: stolen key material, a compromised host or automation, a malicious signing request, and other signing paths cannot be distinguished from chain data alone. No independent host or Ledger forensics was performed. The reporter’s SSH/process-log statements remain unverified. A complete historical role audit and a general audit outside the reviewed scope remain outstanding.

## Wallet and contract identifiers

| Label | Chain | Address | Role |
| --- | --- | --- | --- |
| [VICTIM_A] | Terra Classic | `terra13w2zl4cetngp5cssw0uhkkxryrxn5wqfdgrp2h` | Software wallet, victim-reported; affected source |
| [VICTIM_B] | Terra Classic | `terra12z8vz3vgnalt8trykd6nntdyp54e95yqsdtjj3` | Software wallet, victim-reported; affected source |
| [VICTIM_RECOVERY] | Terra Classic | `terra10jr9whfvrs3kg96409r8vge3mjwqdaejl7jv9d` | Ledger wallet, victim-reported; affected source and later recovery recipient |
| [HACKER_A] | BSC | `0x46b2ebf5ec9bbbca91fb826a6c13aa7ccdb66f8d` | Reported unauthorized BSC destination, seller, and asset holder |
| [HACKER_B] | BSC | `0x377a56e0fea12d5e0632429326b5cd9e47e10e5b` | Reported destination for unsubmitted vFDUSD claim |
| [HACKER_C] | Terra Classic | `terra1gvahgertm8d3ghjmw3ygw958ja99dlyvycm28n` | Reported unauthorized Terra relay and returned-LUNC holder |

| Contract role | Chain | Address |
| --- | --- | --- |
| [FACTORY] | Terra Classic | `terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea` |
| [ROUTER] | Terra Classic | `terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw` |
| [MAPPER] | Terra Classic | `terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2` |
| [TREASURY] | Terra Classic | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` |
| [BRIDGE] | Terra Classic | `terra18m02l2f43c2dagqnz3kfccpgz9pzzz5hk9l5mh5wvr6dcvv47zfqdfs7la` |
| [PAIR1] | Terra Classic | `terra1q5karg99p5hc0jvplgn2fd8akg39r4ss00cu5rwkm600yqvmssgqhwqvq0` |
| [PAIR2] | Terra Classic | `terra16vxrhpvpcucu05y0nr862vf9hnqeh274uaff4s7hz4n0ea74006qf5hgqy` |
| [CL8Y] | Terra Classic | `terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3` |
| [USDT] | Terra Classic | `terra1z0xe7t5ymmltg4vju8tghkq0pewy4et548ta23nlu9zxtl950uyqkv8mv4` |
| [VFDUSD] | Terra Classic | `terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3` |
| [UST1] | Terra Classic | `terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72` |
| [WINDOW] | Terra Classic | `terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2` |
| [ORACLE] | Terra Classic | `terra1fmht0t6svq3n24zx03nkfja0m40zhfyyxkdcvlrkl6u7gfe6aagq4gch8n` |
| [USTR] | Terra Classic | `terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv` |
| [CLUNC] | Terra Classic | `terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg` |
| [CUSTC] | Terra Classic | `terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch` |
| [BSC_BRIDGE] | BSC | `0xb2a22c74da8e3642e0effc107d3ac362ce885369` |
| [BSC_LOCKUNLOCK] | BSC | `0xd7b3bf05987052009c350874e810df98da95d258` |
| [BSC_CL8Y] | BSC | `0x8f452a1fdd388a45e1080992eff051b4dd9048d2` |
| [BSC_USDT] | BSC | `0x55d398326f99059ff775485246999027b3197955` |
| [BSC_WBNB] | BSC | `0xbb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c` |

## Terra → BSC bridge ledger

The nine traced source deposits are nonce 150 and nonces 152–159. Net Terra burns match BSC unlocks or the outstanding claim; those are representations of the same principal. The first row is the previously omitted Ledger-address transfer.

| Nonce | Source UTC | From → destination | Asset | Gross debit | Bridge fee | Net burn/release | Terra transaction | BSC execution |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 150 | 02:10:35 | [VICTIM_RECOVERY] → [HACKER_A] | CL8Y | 8271.070656475910303881 | 24.813211969427730911 | 8246.25744450648257297 | [3C04FDDC18D59E52047AD8AAE3BE5C63D4D1F1DCD1C6F4BD7EF58A07EEAA2C0A](https://finder.terraclassic.community/columbus-5/tx/3C04FDDC18D59E52047AD8AAE3BE5C63D4D1F1DCD1C6F4BD7EF58A07EEAA2C0A) | [0x353999056fd1526c48b1c16ea659ac03b001647161d1873ec08c08df1d546795](https://bscscan.com/tx/0x353999056fd1526c48b1c16ea659ac03b001647161d1873ec08c08df1d546795) |
| 152 | 04:19:38 | [VICTIM_A] → [HACKER_A] | USDT | 237.366032711296597552 | 0.712098098133889792 | 236.65393461316270776 | [E76AF7A330D76A60EC0557347724E2405B07CF1B4878140EAA722AD400A093E7](https://finder.terraclassic.community/columbus-5/tx/E76AF7A330D76A60EC0557347724E2405B07CF1B4878140EAA722AD400A093E7) | [0xae8a074210cecbe91ab70f69f0bac49345ca1d22b4d68fb6e8f261b6ad3885bf](https://bscscan.com/tx/0xae8a074210cecbe91ab70f69f0bac49345ca1d22b4d68fb6e8f261b6ad3885bf) |
| 153 | 04:43:11 | [VICTIM_B] → [HACKER_B] | vFDUSD | 10.184892 | 0.030554 | 10.154338 | [3A5D25F06E769EE61E2E8AC271646A8E0F9F57828102B054F2FB49F635286BAA](https://finder.terraclassic.community/columbus-5/tx/3A5D25F06E769EE61E2E8AC271646A8E0F9F57828102B054F2FB49F635286BAA) | Unsubmitted; not cancelled |
| 154 | 04:43:17 | [VICTIM_B] → [HACKER_A] | USDT | 342.710552514545617774 | 1.028131657543636853 | 341.682420857001980921 | [FDDED73075377ADA5C102C76E53C08FBAB331EA934699CC2D2C3B42C852BF1D7](https://finder.terraclassic.community/columbus-5/tx/FDDED73075377ADA5C102C76E53C08FBAB331EA934699CC2D2C3B42C852BF1D7) | [0xae269ef972e890553c31ea7c76c7b84f356e3ac61b98ee27f1046d28923fe6de](https://bscscan.com/tx/0xae269ef972e890553c31ea7c76c7b84f356e3ac61b98ee27f1046d28923fe6de) |
| 155 | 05:47:46 | [VICTIM_A] → [HACKER_A] | CL8Y | 0.000000000000000001 | 0 | 0.000000000000000001 | [B350B9AE464FA22EED2E17A743C177E72B0BF658F859DE5654D0BA9CFE3472F6](https://finder.terraclassic.community/columbus-5/tx/B350B9AE464FA22EED2E17A743C177E72B0BF658F859DE5654D0BA9CFE3472F6) | [0xa3022923296219ac32db26f094a185a458f210e5b212383e0d88d62066f52ff5](https://bscscan.com/tx/0xa3022923296219ac32db26f094a185a458f210e5b212383e0d88d62066f52ff5) |
| 156 | 05:53:41 | [VICTIM_A] → [HACKER_A] | CL8Y | 9900 | 29.7 | 9870.3 | [5AAF95217E21E94BC7089750EA00EE5AC32E0E39D1F22AC95E61EB5A39C18824](https://finder.terraclassic.community/columbus-5/tx/5AAF95217E21E94BC7089750EA00EE5AC32E0E39D1F22AC95E61EB5A39C18824) | [0xad30847f9c77cac1f9405483beca52516814422534b0e030a9a3131e653e679b](https://bscscan.com/tx/0xad30847f9c77cac1f9405483beca52516814422534b0e030a9a3131e653e679b) |
| 157 | 05:54:47 | [VICTIM_A] → [HACKER_A] | CL8Y | 2454.036524384209506669 | 7.36210957315262852 | 2446.674414811056878149 | [E004EEE6DBEA8DFEB0E45C9766E002F6E836F3FBA47B66086B4CC21E52E31132](https://finder.terraclassic.community/columbus-5/tx/E004EEE6DBEA8DFEB0E45C9766E002F6E836F3FBA47B66086B4CC21E52E31132) | [0xb32eae046d86c473eb6eb27278199161a64f521db12b16831762d0f562b61586](https://bscscan.com/tx/0xb32eae046d86c473eb6eb27278199161a64f521db12b16831762d0f562b61586) |
| 158 | 06:01:29 | [VICTIM_B] → [HACKER_A] | CL8Y | 9000 | 27 | 8973 | [A418EABA2E5C50A19938791F37FD414F1416EA204C535B46B157E63DED9637E4](https://finder.terraclassic.community/columbus-5/tx/A418EABA2E5C50A19938791F37FD414F1416EA204C535B46B157E63DED9637E4) | [0x87022cf8a5144ccbabc272f1f68f6b8dbf97130e9022c8504294bdd5a3ae51b7](https://bscscan.com/tx/0x87022cf8a5144ccbabc272f1f68f6b8dbf97130e9022c8504294bdd5a3ae51b7) |
| 159 | 06:01:59 | [VICTIM_B] → [HACKER_A] | CL8Y | 2523.843619896252112646 | 7.571530859688756337 | 2516.272089036563356309 | [4334416A150C402ED31DC435A58F8DDB2E227372FA8CE54BD7138A0444B608B9](https://finder.terraclassic.community/columbus-5/tx/4334416A150C402ED31DC435A58F8DDB2E227372FA8CE54BD7138A0444B608B9) | [0x1265ac05190b12adb0daa3fd94496207409347f836594cf916184fbf3a2ff0f0](https://bscscan.com/tx/0x1265ac05190b12adb0daa3fd94496207409347f836594cf916184fbf3a2ff0f0) |

Bridge fees went to source protocol fee collector `terra1q7txczaxuvy923k4km9ya062dryk6mjwd6tmzm`; this is not an attacker-attribution finding.

| Asset | Gross source debit | Source bridge fee | Net burned / claim or release |
| --- | --- | --- | --- |
| CL8Y | 32148.950800756371923197 | 96.446852402269115768 | 32052.503948354102807429 |
| USDT | 580.076585225842215326 | 1.740229755677526645 | 578.336355470164688681 |
| vFDUSD | 10.184892 | 0.030554 | 10.154338 |

## Earlier 20 sales: CL8Y debits and actual BNB payouts

Each row is a successful transaction. Fees are included inside the CL8Y debit. WBNB unwrapped is transit, not a second wallet profit. Every native payout is verified from its successful call tree, with reverted branches and DELEGATECALL apparent values excluded.

| UTC | Account nonce | CL8Y debited | CL8Y fee inside debit | BNB credited to wallet | WBNB unwrapped in route | BSC transaction |
| --- | --- | --- | --- | --- | --- | --- |
| 02:28:45 | 2 | 500 | 1.25 | 0.393303312709113685 | 0.393303312709113685 | [0x867120ba093c958ddee63fd56246059734db3f5a3c09b8ee04e55c205b3546ed](https://bscscan.com/tx/0x867120ba093c958ddee63fd56246059734db3f5a3c09b8ee04e55c205b3546ed) |
| 02:29:02 | 4 | 500 | 1.25 | 0.387953431482512255 | 0.387953431482512255 | [0x9952d0c7a5008e2eeafbce5ee034dba5655d646d46fdafecacb30dbaa7973d10](https://bscscan.com/tx/0x9952d0c7a5008e2eeafbce5ee034dba5655d646d46fdafecacb30dbaa7973d10) |
| 02:29:17 | 6 | 500 | 1.25 | 0.382575600440050872 | 0.382575600440050872 | [0x8ab302c41325b48bd920e368b154e9bf67d78410a2c87432b0417eca909b31be](https://bscscan.com/tx/0x8ab302c41325b48bd920e368b154e9bf67d78410a2c87432b0417eca909b31be) |
| 02:29:31 | 8 | 500 | 1.25 | 0.345490377369393633 | 0.345490377369393633 | [0x875bc1607206665379f746f01c4cc8571f8c9b6db52ab3b809f226a258d8acab](https://bscscan.com/tx/0x875bc1607206665379f746f01c4cc8571f8c9b6db52ab3b809f226a258d8acab) |
| 02:30:24 | 10 | 500 | 1.25 | 0.310188225892298639 | 0.310188225892298639 | [0xb1e1cb550c32576717ef6acd606c8f55ca096e557dc8dd557a85f9d9a46d8621](https://bscscan.com/tx/0xb1e1cb550c32576717ef6acd606c8f55ca096e557dc8dd557a85f9d9a46d8621) |
| 02:30:52 | 12 | 500 | 1.25 | 0.278145258824276987 | 0.278145258824276987 | [0xfb85709c2bb4a211259cccb55b4bc4318bec897d9377aee317907a79b1fe4166](https://bscscan.com/tx/0xfb85709c2bb4a211259cccb55b4bc4318bec897d9377aee317907a79b1fe4166) |
| 02:31:27 | 14 | 500 | 1.25 | 0.250797999317700429 | 0.250797999317700429 | [0x6ead31f984cddaed375ce1793e884a292f0b579690c12365f52d6892f278c0f9](https://bscscan.com/tx/0x6ead31f984cddaed375ce1793e884a292f0b579690c12365f52d6892f278c0f9) |
| 02:31:54 | 16 | 250 | 0.625 | 0.184858330738127676 | 0.184858330738127676 | [0x7286454192a17907476f049e34f350da9c25deeca55cb449277c88f5d70f4b67](https://bscscan.com/tx/0x7286454192a17907476f049e34f350da9c25deeca55cb449277c88f5d70f4b67) |
| 02:32:40 | 18 | 250 | 0.625 | 0.183394378301260006 | 0.183394378301260006 | [0x72587c94cacab93c19bf6c523a8a9b5e630539c53c6670d07af3273badada8e1](https://bscscan.com/tx/0x72587c94cacab93c19bf6c523a8a9b5e630539c53c6670d07af3273badada8e1) |
| 02:32:54 | 20 | 250 | 0.625 | 0.181497617738808626 | 0.181497617738808626 | [0x406e037d96112c79618f5ebec5c0664f2b1ce99a75c2187d97ef2d26d4af4947](https://bscscan.com/tx/0x406e037d96112c79618f5ebec5c0664f2b1ce99a75c2187d97ef2d26d4af4947) |
| 02:33:11 | 22 | 250 | 0.625 | 0.174295384509284012 | 0.174295384509284012 | [0x179e27a82b15325e4da67d165f52df3c2a5cc349fb352a2fa058dc0a7e190489](https://bscscan.com/tx/0x179e27a82b15325e4da67d165f52df3c2a5cc349fb352a2fa058dc0a7e190489) |
| 02:34:23 | 24 | 250 | 0.625 | 0.152720513496877755 | 0.152720513496877755 | [0xeb0c665f03485cc12f3d5213dabebb57b5dabd2547a44c8dbbec6bfac574a962](https://bscscan.com/tx/0xeb0c665f03485cc12f3d5213dabebb57b5dabd2547a44c8dbbec6bfac574a962) |
| 02:34:34 | 26 | 250 | 0.625 | 0.139777970107439185 | 0.138266132032051651 | [0xd1c8870561f29e29e730b2d91cf471d7634a9b29c405132f59fcdc6cf3a04824](https://bscscan.com/tx/0xd1c8870561f29e29e730b2d91cf471d7634a9b29c405132f59fcdc6cf3a04824) |
| 02:40:45 | 35 | 250 | 0.625 | 0.120751298548562176 | 0.118774184618103693 | [0xfe478847369d9297d9c74a7dac969d1718288cea07bc942dd9604a6140d1a11d](https://bscscan.com/tx/0xfe478847369d9297d9c74a7dac969d1718288cea07bc942dd9604a6140d1a11d) |
| 02:41:03 | 37 | 250 | 0.625 | 0.100032645518428848 | 0.100032645518428848 | [0xe2756eccc11f3701253c8af260e279b089f1d338defdcd42724dfda6288d589a](https://bscscan.com/tx/0xe2756eccc11f3701253c8af260e279b089f1d338defdcd42724dfda6288d589a) |
| 02:41:14 | 39 | 250 | 0.625 | 0.090818928799916949 | 0.088725266022670655 | [0xcff26cfadfcc923fd743ebac33a914ad353648e20879a40b3e1ffb1d70cef191](https://bscscan.com/tx/0xcff26cfadfcc923fd743ebac33a914ad353648e20879a40b3e1ffb1d70cef191) |
| 02:42:42 | 41 | 250 | 0.625 | 0.081841279703150911 | 0.081841279703150911 | [0x93c143c7ac643084d2901beaafc04d0a6fc1e14691eead85f11d79f1f9e063e8](https://bscscan.com/tx/0x93c143c7ac643084d2901beaafc04d0a6fc1e14691eead85f11d79f1f9e063e8) |
| 02:42:52 | 43 | 250 | 0.625 | 0.072951297648473983 | 0.072926684585294894 | [0xef15471c78b91193c90891edb9447420a19d5ae4b953615ce51af805e06c7f7c](https://bscscan.com/tx/0xef15471c78b91193c90891edb9447420a19d5ae4b953615ce51af805e06c7f7c) |
| 02:43:13 | 45 | 250 | 0.625 | 0.055524631933801524 | 0.055501935156914741 | [0x76a295ca2eeef1969199f0248926a8a4b4efb0024f90da74ca8228eada86e503](https://bscscan.com/tx/0x76a295ca2eeef1969199f0248926a8a4b4efb0024f90da74ca8228eada86e503) |
| 02:49:56 | 47 | 100 | 0.25 | 0.056520693434057547 | 0.056264062964392945 | [0xa25a983d076c1f6e9e64d564114322ce95ea0cd6782b936e43b73450aacd3915](https://bscscan.com/tx/0xa25a983d076c1f6e9e64d564114322ce95ea0cd6782b936e43b73450aacd3915) |

## Native BNB funding and all outgoing transactions

The three incoming funding sources are un-attributed. Their payments do not establish attacker ownership or additional victims.

| UTC | Funding label | From address | BNB credited to `[HACKER_A]` | BSC transaction |
| --- | --- | --- | --- | --- |
| 02:13:08 | [UNATTRIBUTED_FUNDING_SOURCE_1] | `0x91604f590d66ace8975eed6bd16cf55647d1c499` | 0.000027340988674999 | [0x2f6cfa2d665d8f55745c7b0a70e1ccdbfa215c74a4e875ce9461e81544825cd9](https://bscscan.com/tx/0x2f6cfa2d665d8f55745c7b0a70e1ccdbfa215c74a4e875ce9461e81544825cd9) |
| 02:28:43 | [UNATTRIBUTED_FUNDING_SOURCE_2] | `0x76dd65529dc6c073c1e0af2a5ecc78434bdbf7d9` | 0.000027181594577501 | [0xab064a224dca4def316aa0f4215c8d5c1c2e6051b5b73a4f23794f6df25f4317](https://bscscan.com/tx/0xab064a224dca4def316aa0f4215c8d5c1c2e6051b5b73a4f23794f6df25f4317) |
| 02:41:20 | [UNATTRIBUTED_FUNDING_SOURCE_3] | `0x3b2d4046a86b493d213b847548d8bd42a13a7bf9` | 0.000008395865076607 | [0x0944bfa82cb04a39bc325cca5b1ee513885aba07678591c502a103c7f6a6a7bc](https://bscscan.com/tx/0x0944bfa82cb04a39bc325cca5b1ee513885aba07678591c502a103c7f6a6a7bc) |

All 56 outgoing transactions, including failed swaps and approvals:

| UTC | Nonce | Action | Status | BNB gas paid | BSC transaction |
| --- | --- | --- | --- | --- | --- |
| 02:13:09 | 0 | withdrawSubmit | 1 | 0.0000161619552 | [0x871b910906b6f94cd5d83c439feb2af4493a0077f1968b8dafdc878ea1b6b8db](https://bscscan.com/tx/0x871b910906b6f94cd5d83c439feb2af4493a0077f1968b8dafdc878ea1b6b8db) |
| 02:28:38 | 1 | approve 500 CL8Y to 0x111111125421ca6dc452d289314280a0f8842a65 | 1 | 0.0000024819781 | [0x67aaa2908fecddd8aba72520770fc8a76c3e50611d61632048c67f754d5f8ace](https://bscscan.com/tx/0x67aaa2908fecddd8aba72520770fc8a76c3e50611d61632048c67f754d5f8ace) |
| 02:28:45 | 2 | Successful sale | 1 | 0.0000197678306 | [0x867120ba093c958ddee63fd56246059734db3f5a3c09b8ee04e55c205b3546ed](https://bscscan.com/tx/0x867120ba093c958ddee63fd56246059734db3f5a3c09b8ee04e55c205b3546ed) |
| 02:29:02 | 3 | approve 500 CL8Y to 0x111111125421ca6dc452d289314280a0f8842a65 | 1 | 0.00000251986348 | [0x6f20b4e87ae70888e0011f63da7e9e3aafa53dfd2a98e1c2a8d655781adf344c](https://bscscan.com/tx/0x6f20b4e87ae70888e0011f63da7e9e3aafa53dfd2a98e1c2a8d655781adf344c) |
| 02:29:02 | 4 | Successful sale | 1 | 0.00002007382464 | [0x9952d0c7a5008e2eeafbce5ee034dba5655d646d46fdafecacb30dbaa7973d10](https://bscscan.com/tx/0x9952d0c7a5008e2eeafbce5ee034dba5655d646d46fdafecacb30dbaa7973d10) |
| 02:29:16 | 5 | approve 500 CL8Y to 0x111111125421ca6dc452d289314280a0f8842a65 | 1 | 0.000002525692 | [0x8effa5357450cefbb04554f284da7e5c35e20405ee61c36a00d7a6aeaab0767b](https://bscscan.com/tx/0x8effa5357450cefbb04554f284da7e5c35e20405ee61c36a00d7a6aeaab0767b) |
| 02:29:17 | 6 | Successful sale | 1 | 0.000020135492 | [0x8ab302c41325b48bd920e368b154e9bf67d78410a2c87432b0417eca909b31be](https://bscscan.com/tx/0x8ab302c41325b48bd920e368b154e9bf67d78410a2c87432b0417eca909b31be) |
| 02:29:30 | 7 | approve 500 CL8Y to 0x20f6ee51340adeed01a59b0e65cb3703f3dc860c | 1 | 0.000002525692 | [0xc57cde6b49dfd8bd9cb3b08dcfee0adf98b45597769377dbb0c4c3797eb69b3e](https://bscscan.com/tx/0xc57cde6b49dfd8bd9cb3b08dcfee0adf98b45597769377dbb0c4c3797eb69b3e) |
| 02:29:31 | 8 | Successful sale | 1 | 0.000062032464 | [0x875bc1607206665379f746f01c4cc8571f8c9b6db52ab3b809f226a258d8acab](https://bscscan.com/tx/0x875bc1607206665379f746f01c4cc8571f8c9b6db52ab3b809f226a258d8acab) |
| 02:30:22 | 9 | approve 500 CL8Y to 0x6131b5fae19ea4f9d964eac0408e4408b66337b5 | 1 | 0.000002477121 | [0xd21f0c4931a924d47d0dcbb866b5decdea72f51afa3cfc1d50bee015f3578336](https://bscscan.com/tx/0xd21f0c4931a924d47d0dcbb866b5decdea72f51afa3cfc1d50bee015f3578336) |
| 02:30:24 | 10 | Successful sale | 1 | 0.000037702158 | [0xb1e1cb550c32576717ef6acd606c8f55ca096e557dc8dd557a85f9d9a46d8621](https://bscscan.com/tx/0xb1e1cb550c32576717ef6acd606c8f55ca096e557dc8dd557a85f9d9a46d8621) |
| 02:30:50 | 11 | approve 500 CL8Y to 0x6131b5fae19ea4f9d964eac0408e4408b66337b5 | 1 | 0.0000024819781 | [0x7e1070eb848fc0f643eda2e3a2b419a142e7ca213438a2ba3f1f2c0031051aa1](https://bscscan.com/tx/0x7e1070eb848fc0f643eda2e3a2b419a142e7ca213438a2ba3f1f2c0031051aa1) |
| 02:30:52 | 12 | Successful sale | 1 | 0.0000346255644 | [0xfb85709c2bb4a211259cccb55b4bc4318bec897d9377aee317907a79b1fe4166](https://bscscan.com/tx/0xfb85709c2bb4a211259cccb55b4bc4318bec897d9377aee317907a79b1fe4166) |
| 02:31:24 | 13 | approve 500 CL8Y to 0x6131b5fae19ea4f9d964eac0408e4408b66337b5 | 1 | 0.0000024916923 | [0x6b34b43c9247213a338fc83477681159083cef540595a64018ff6aee5fea2131](https://bscscan.com/tx/0x6b34b43c9247213a338fc83477681159083cef540595a64018ff6aee5fea2131) |
| 02:31:27 | 14 | Successful sale | 1 | 0.0000391539555 | [0x6ead31f984cddaed375ce1793e884a292f0b579690c12365f52d6892f278c0f9](https://bscscan.com/tx/0x6ead31f984cddaed375ce1793e884a292f0b579690c12365f52d6892f278c0f9) |
| 02:31:52 | 15 | approve 250 CL8Y to 0x111111125421ca6dc452d289314280a0f8842a65 | 1 | 0.0000024819781 | [0xd9c6e8aa057b07e07520036e10b1ff038eea96fad3b77cdb743ef0f5856e4ebc](https://bscscan.com/tx/0xd9c6e8aa057b07e07520036e10b1ff038eea96fad3b77cdb743ef0f5856e4ebc) |
| 02:31:54 | 16 | Successful sale | 1 | 0.0000182313558 | [0x7286454192a17907476f049e34f350da9c25deeca55cb449277c88f5d70f4b67](https://bscscan.com/tx/0x7286454192a17907476f049e34f350da9c25deeca55cb449277c88f5d70f4b67) |
| 02:32:38 | 17 | approve 250 CL8Y to 0x111111125421ca6dc452d289314280a0f8842a65 | 1 | 0.000002477121 | [0xa446931749ede656b8565da39c40638dcf3e9c0a3081d1098d45f58b17442343](https://bscscan.com/tx/0xa446931749ede656b8565da39c40638dcf3e9c0a3081d1098d45f58b17442343) |
| 02:32:40 | 18 | Successful sale | 1 | 0.000018195882 | [0x72587c94cacab93c19bf6c523a8a9b5e630539c53c6670d07af3273badada8e1](https://bscscan.com/tx/0x72587c94cacab93c19bf6c523a8a9b5e630539c53c6670d07af3273badada8e1) |
| 02:32:52 | 19 | approve 250 CL8Y to 0x111111125421ca6dc452d289314280a0f8842a65 | 1 | 0.000002477121 | [0x150ad319f7c4a4d8f596678795d4fdcb7b121ff48038b4e93bfffb25e660b7ef](https://bscscan.com/tx/0x150ad319f7c4a4d8f596678795d4fdcb7b121ff48038b4e93bfffb25e660b7ef) |
| 02:32:54 | 20 | Successful sale | 1 | 0.000018216945 | [0x406e037d96112c79618f5ebec5c0664f2b1ce99a75c2187d97ef2d26d4af4947](https://bscscan.com/tx/0x406e037d96112c79618f5ebec5c0664f2b1ce99a75c2187d97ef2d26d4af4947) |
| 02:33:10 | 21 | approve 250 CL8Y to 0x6131b5fae19ea4f9d964eac0408e4408b66337b5 | 1 | 0.000002477121 | [0x4f0d852a2d5170f59f0d9805a598a4ad3ff994ae8c18e2ed5eec32e6ede9f819](https://bscscan.com/tx/0x4f0d852a2d5170f59f0d9805a598a4ad3ff994ae8c18e2ed5eec32e6ede9f819) |
| 02:33:11 | 22 | Successful sale | 1 | 0.000036543234 | [0x179e27a82b15325e4da67d165f52df3c2a5cc349fb352a2fa058dc0a7e190489](https://bscscan.com/tx/0x179e27a82b15325e4da67d165f52df3c2a5cc349fb352a2fa058dc0a7e190489) |
| 02:34:22 | 23 | approve 250 CL8Y to 0x6131b5fae19ea4f9d964eac0408e4408b66337b5 | 1 | 0.000002477121 | [0x664871bfe61427464dcf8d0f042a718cf903b8f958303b8ad293f05bbc85edc1](https://bscscan.com/tx/0x664871bfe61427464dcf8d0f042a718cf903b8f958303b8ad293f05bbc85edc1) |
| 02:34:23 | 24 | Successful sale | 1 | 0.000038149581 | [0xeb0c665f03485cc12f3d5213dabebb57b5dabd2547a44c8dbbec6bfac574a962](https://bscscan.com/tx/0xeb0c665f03485cc12f3d5213dabebb57b5dabd2547a44c8dbbec6bfac574a962) |
| 02:34:32 | 25 | approve 250 CL8Y to 0x6131b5fae19ea4f9d964eac0408e4408b66337b5 | 1 | 0.000002477121 | [0xc9a4243dd90d1374002de3afd01a638ac4f80c813a55dcdc805c5c28ee86d6e0](https://bscscan.com/tx/0xc9a4243dd90d1374002de3afd01a638ac4f80c813a55dcdc805c5c28ee86d6e0) |
| 02:34:34 | 26 | Successful sale | 1 | 0.000039948606 | [0xd1c8870561f29e29e730b2d91cf471d7634a9b29c405132f59fcdc6cf3a04824](https://bscscan.com/tx/0xd1c8870561f29e29e730b2d91cf471d7634a9b29c405132f59fcdc6cf3a04824) |
| 02:37:12 | 27 | approve 250 CL8Y to 0x6352a56caadc4f1e25cd6c75970fa768a3304e64 | 1 | 0.000002477121 | [0x9b699433530275bff447268facc357848df124ebcd8ca128553979f4808cf40f](https://bscscan.com/tx/0x9b699433530275bff447268facc357848df124ebcd8ca128553979f4808cf40f) |
| 02:37:14 | 28 | Failed swap attempt | 0 | 0.000791735883 | [0x256587ce727b98e6b68caee4f8ebf1ce57921d82ac06c9eba6beaf18477c9501](https://bscscan.com/tx/0x256587ce727b98e6b68caee4f8ebf1ce57921d82ac06c9eba6beaf18477c9501) |
| 02:37:35 | 29 | Failed swap attempt | 0 | 0.00079173675 | [0x06269ab8aebd601d3088f2a9c6c970bf672abd8c4e13b92d8098903e8f715632](https://bscscan.com/tx/0x06269ab8aebd601d3088f2a9c6c970bf672abd8c4e13b92d8098903e8f715632) |
| 02:37:47 | 30 | Failed swap attempt | 0 | 0.000791735883 | [0xe191abd8dd1e5438885d1d201038542eb819996bd0b9fb6e20a642c3685d2b47](https://bscscan.com/tx/0xe191abd8dd1e5438885d1d201038542eb819996bd0b9fb6e20a642c3685d2b47) |
| 02:37:58 | 31 | Failed swap attempt | 0 | 0.000791735781 | [0xf224453ac1bf098df08eae9305299bc70a82bbe217567bc50ac10fa57741d711](https://bscscan.com/tx/0xf224453ac1bf098df08eae9305299bc70a82bbe217567bc50ac10fa57741d711) |
| 02:38:11 | 32 | Failed swap attempt | 0 | 0.000791735883 | [0xa370b2e4d692f7da5b8218f49c3ae012d7acf901876dc2860ba2cf4cef1c5baf](https://bscscan.com/tx/0xa370b2e4d692f7da5b8218f49c3ae012d7acf901876dc2860ba2cf4cef1c5baf) |
| 02:38:44 | 33 | Failed swap attempt | 0 | 0.000791788464 | [0x0f904880dec40de9b825da33614f8001c494de0482a0f74690360a6d75af30da](https://bscscan.com/tx/0x0f904880dec40de9b825da33614f8001c494de0482a0f74690360a6d75af30da) |
| 02:40:44 | 34 | approve 250 CL8Y to 0xac4c6e212a361c968f1725b4d055b47e63f80b75 | 1 | 0.0000024916923 | [0xa81fa6417dd5e7dbc122706ab6a52a61034a9a0d5e7994c763b1294ff6746a5a](https://bscscan.com/tx/0xa81fa6417dd5e7dbc122706ab6a52a61034a9a0d5e7994c763b1294ff6746a5a) |
| 02:40:45 | 35 | Successful sale | 1 | 0.0000294897537 | [0xfe478847369d9297d9c74a7dac969d1718288cea07bc942dd9604a6140d1a11d](https://bscscan.com/tx/0xfe478847369d9297d9c74a7dac969d1718288cea07bc942dd9604a6140d1a11d) |
| 02:41:01 | 36 | approve 250 CL8Y to 0x20f6ee51340adeed01a59b0e65cb3703f3dc860c | 1 | 0.0000024916923 | [0xe7611fd829fc82f5dc7996f27e315142b352c4fe1997d86a624ebd19d5c6cfc5](https://bscscan.com/tx/0xe7611fd829fc82f5dc7996f27e315142b352c4fe1997d86a624ebd19d5c6cfc5) |
| 02:41:03 | 37 | Successful sale | 1 | 0.000161342091 | [0xe2756eccc11f3701253c8af260e279b089f1d338defdcd42724dfda6288d589a](https://bscscan.com/tx/0xe2756eccc11f3701253c8af260e279b089f1d338defdcd42724dfda6288d589a) |
| 02:41:13 | 38 | approve 250 CL8Y to 0x20f6ee51340adeed01a59b0e65cb3703f3dc860c | 1 | 0.00000251646351 | [0x69df3051dd627ff5df1b9af9dbc511f4fd49269e7338dd02a5de5d2d6fad2dea](https://bscscan.com/tx/0x69df3051dd627ff5df1b9af9dbc511f4fd49269e7338dd02a5de5d2d6fad2dea) |
| 02:41:14 | 39 | Successful sale | 1 | 0.0001033443708 | [0xcff26cfadfcc923fd743ebac33a914ad353648e20879a40b3e1ffb1d70cef191](https://bscscan.com/tx/0xcff26cfadfcc923fd743ebac33a914ad353648e20879a40b3e1ffb1d70cef191) |
| 02:42:40 | 40 | approve 250 CL8Y to 0x20f6ee51340adeed01a59b0e65cb3703f3dc860c | 1 | 0.000002525692 | [0xb9442f48dd11e8e1065a7918e54690937d280f7364aa1aa9081c5cd698fda7ab](https://bscscan.com/tx/0xb9442f48dd11e8e1065a7918e54690937d280f7364aa1aa9081c5cd698fda7ab) |
| 02:42:42 | 41 | Successful sale | 1 | 0.00016478956 | [0x93c143c7ac643084d2901beaafc04d0a6fc1e14691eead85f11d79f1f9e063e8](https://bscscan.com/tx/0x93c143c7ac643084d2901beaafc04d0a6fc1e14691eead85f11d79f1f9e063e8) |
| 02:42:51 | 42 | approve 250 CL8Y to 0x20f6ee51340adeed01a59b0e65cb3703f3dc860c | 1 | 0.000002525692 | [0x0634a357395836e6391b74ec4ad5e701c6905e48d07632c1700f5ba670914763](https://bscscan.com/tx/0x0634a357395836e6391b74ec4ad5e701c6905e48d07632c1700f5ba670914763) |
| 02:42:52 | 43 | Successful sale | 1 | 0.000147423016 | [0xef15471c78b91193c90891edb9447420a19d5ae4b953615ce51af805e06c7f7c](https://bscscan.com/tx/0xef15471c78b91193c90891edb9447420a19d5ae4b953615ce51af805e06c7f7c) |
| 02:43:12 | 44 | approve 250 CL8Y to 0x20f6ee51340adeed01a59b0e65cb3703f3dc860c | 1 | 0.000002525692 | [0x16644842cbb9e90af4bf1a3835c9c7ebf57873ffeae41e1481e9e8f569aea526](https://bscscan.com/tx/0x16644842cbb9e90af4bf1a3835c9c7ebf57873ffeae41e1481e9e8f569aea526) |
| 02:43:13 | 45 | Successful sale | 1 | 0.000203444488 | [0x76a295ca2eeef1969199f0248926a8a4b4efb0024f90da74ca8228eada86e503](https://bscscan.com/tx/0x76a295ca2eeef1969199f0248926a8a4b4efb0024f90da74ca8228eada86e503) |
| 02:49:54 | 46 | approve 100 CL8Y to 0x20f6ee51340adeed01a59b0e65cb3703f3dc860c | 1 | 0.00000257134874 | [0xedbd63edaf07f3a950b195c51950750680f886b19616d956e29df0cc6a1c2612](https://bscscan.com/tx/0xedbd63edaf07f3a950b195c51950750680f886b19616d956e29df0cc6a1c2612) |
| 02:49:56 | 47 | Successful sale | 1 | 0.00005137991114 | [0xa25a983d076c1f6e9e64d564114322ce95ea0cd6782b936e43b73450aacd3915](https://bscscan.com/tx/0xa25a983d076c1f6e9e64d564114322ce95ea0cd6782b936e43b73450aacd3915) |
| 04:19:50 | 48 | withdrawSubmit | 1 | 0.000016472983 | [0x8c33f59ce6527fc4a510ee73fa5eb653e9f666ca7d09a70e204e6506c3514b27](https://bscscan.com/tx/0x8c33f59ce6527fc4a510ee73fa5eb653e9f666ca7d09a70e204e6506c3514b27) |
| 04:43:27 | 49 | withdrawSubmit | 1 | 0.000017094605 | [0x18cbfe4359c8ed9e7c365f93985fc383252b01cb13d1d7c0f14b4e59105e4429](https://bscscan.com/tx/0x18cbfe4359c8ed9e7c365f93985fc383252b01cb13d1d7c0f14b4e59105e4429) |
| 05:47:59 | 50 | withdrawSubmit | 1 | 0.00001622426832 | [0x0bf283f186fe5ceaff3cd1014369c99070eff479617807ad810f68174e972198](https://bscscan.com/tx/0x0bf283f186fe5ceaff3cd1014369c99070eff479617807ad810f68174e972198) |
| 05:53:57 | 51 | withdrawSubmit | 1 | 0.000016068 | [0x5c9f89f020a6c24d46a0f4bea97eed7d11d4463d9600c5564009dc7514251a74](https://bscscan.com/tx/0x5c9f89f020a6c24d46a0f4bea97eed7d11d4463d9600c5564009dc7514251a74) |
| 05:54:59 | 52 | withdrawSubmit | 1 | 0.000016068624 | [0x3c38202f74a09dec0f616855f9cb4769bd9ff35831f44c960635ecfb913e53f8](https://bscscan.com/tx/0x3c38202f74a09dec0f616855f9cb4769bd9ff35831f44c960635ecfb913e53f8) |
| 06:01:40 | 53 | withdrawSubmit | 1 | 0.000017304 | [0x1443281ed6cea8778f2563539212d817adad48e01eb0c252c10ca3fe23a622d4](https://bscscan.com/tx/0x1443281ed6cea8778f2563539212d817adad48e01eb0c252c10ca3fe23a622d4) |
| 06:02:10 | 54 | withdrawSubmit | 1 | 0.000017613684 | [0xa3e94c54ddc6f49124f54099061a2bc18ae53ee101d1ae685652f1e12a4d9200](https://bscscan.com/tx/0xa3e94c54ddc6f49124f54099061a2bc18ae53ee101d1ae685652f1e12a4d9200) |
| 06:06:59 | 55 | approve 1000 CL8Y to 0x84d88856a62ce42165d3f196124d5bd50f17c2bb | 1 | 0.00000253977759 | [0xb9086ae2acaf5ce4ac47cece5dae1594d0c99a63cc2f6064774cbf4d9519245f](https://bscscan.com/tx/0xb9086ae2acaf5ce4ac47cece5dae1594d0c99a63cc2f6064774cbf4d9519245f) |

## Native USTC/LUNC route

| UTC | Sender | Asset | Gross debit | Chain tax | Credit to [HACKER_C] | Separate gas, LUNC | Terra transaction |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 05:35:27 | [VICTIM_A] | USTC | 44700 | 670.5 | 44029.5 | 10.451132 | [88D1A861F775079ED7DEACD31028EB5B2B25E2FF6DCD9E8EE7E9554D70C4D9DB](https://finder.terraclassic.community/columbus-5/tx/88D1A861F775079ED7DEACD31028EB5B2B25E2FF6DCD9E8EE7E9554D70C4D9DB) |
| 05:35:45 | [VICTIM_A] | LUNC | 1270000 | 19050 | 1250950 | 9.289807 | [D9BE15AE716C1977F822CBA2C5947C181AFAFF25FEE2DE4A586D96453E475FE2](https://finder.terraclassic.community/columbus-5/tx/D9BE15AE716C1977F822CBA2C5947C181AFAFF25FEE2DE4A586D96453E475FE2) |
| 05:36:03 | [VICTIM_B] | LUNC | 5160000 | 77400 | 5082600 | 9.287173 | [2C5F233DA80C42B0A1614F3B9A6BBE1E3083E4EE73452F955DDCEB109BAF4BC4](https://finder.terraclassic.community/columbus-5/tx/2C5F233DA80C42B0A1614F3B9A6BBE1E3083E4EE73452F955DDCEB109BAF4BC4) |

### USTC → Osmosis pool 562 → LUNC → Terra return

1. **06:02:46 UTC — outbound IBC.** `[HACKER_C]` sends **44029.5 USTC** through Terra `transfer/channel-1`, packet **395425**, to Osmosis `transfer/channel-72` and execution contract `osmo10a3k4hvk37cc4hnxctw4p95fhscd2z6h2rmx0aukc6rm8u9qqx9smfsh7u`. The Terra relay pays **4.284242 LUNC** gas separately. Source transaction: [58EBD8FD50CB9906DE6C90F05C08FCF72B6175316D4A9DCEC8AC22E1740488DA](https://finder.terraclassic.community/columbus-5/tx/58EBD8FD50CB9906DE6C90F05C08FCF72B6175316D4A9DCEC8AC22E1740488DA).
2. **06:03:17 UTC — successful Osmosis execution.** Input routing takes **44.0295 USTC** to `osmo1r9jc2234fljy93z80cevqjt3nmjycec8aj4cc6`; the remaining **43985.4705 USTC** is delivered through swap adapter `osmo1xzphwrc9aahmq7smwaggukwx5w2w8mmtqf5mk0ky0v6hh75paxsszfnlu5` to pool **562**, address `osmo1yggqusvjqscjy03ysdmjh5hxmt6tv20u5ufrsuy67ksnw2xqsvzql2j70s`. Pool output is exactly **4353371.354935 LUNC**. Transaction: [3A389144932AA7104C1081E1ED30497EC2B5C027A8FBC0294C6108E94A1A0087](https://www.mintscan.io/osmosis/tx/3A389144932AA7104C1081E1ED30497EC2B5C027A8FBC0294C6108E94A1A0087).
3. **Same Osmosis transaction — output distribution.** Contract `osmo10a3k4hvk37cc4hnxctw4p95fhscd2z6h2rmx0aukc6rm8u9qqx9smfsh7u` pays **22165.299087 LUNC** to `osmo1gy30axzt9djlqnykmmqdjn4z670ctxsyr97pvf` and **6999.568133 LUNC** to `osmo14gf6xslwe9phn2z965t4dcu7vchhthfgv2lhyy` as the memo's affiliate-route payouts. Those recipients are not attributed as attackers. Their amounts match floor-rounded 57-bps and 18-bps charges against the memo's **3888648.962795 LUNC minimum-output reference**, rather than percentages of actual pool output. The observed output split is **4353371.354935 − 22165.299087 − 6999.568133 = 4324206.487715 LUNC**. Pool trading fees/spread are embedded in the actual pool output; no separate unobserved fee is inferred.
4. **Return IBC.** The exact **4324206.487715 LUNC** passes through return adapter `osmo1m8wg4vxkefhs374qxmmqpyusgz289wmulex5qdwpfx7jnrxzer5s9cv83q`, Osmosis `transfer/channel-72` packet **583917**, to Terra `transfer/channel-1` with `[HACKER_C]` as receiver.
5. **06:03:27 UTC — confirmed Terra receipt.** [339289860E63E23D93F7C89E7A3D8B2DE37DCBE9DC2C8DF01EB3620428B497E4](https://finder.terraclassic.community/columbus-5/tx/339289860E63E23D93F7C89E7A3D8B2DE37DCBE9DC2C8DF01EB3620428B497E4) credits `[HACKER_C]` **4324206.487715 native LUNC**. Packet matching and acknowledgement establish a completed return, not merely an intended route.

The Osmosis relayer pays **0.120637 OSMO** transaction gas; that is not a debit from the tracked victim/attacker wallets. The same transaction also records a separate **protorev** module backrun with **66.635398 OSMO** profit. That protocol-module profit is excluded from attacker proceeds and from the incident-specific transfer ledger; sharing the transaction hash does not make it a payment to `[HACKER_C]`.

Exact relay LUNC accounting: **1250950 + 5082600 + 4324206.487715 − 4.284242 = 10657752.203473 LUNC**. Current captured USTC at `[HACKER_C]` is zero. Incident-specific Osmosis transfer edges, including the fee recipients and every routing contract: [osmosis-asset-flow-ledger.csv](ledgers/osmosis-asset-flow-ledger.csv).


## Owner-described transfers to the affected Ledger address

These receipts establish what reached `[VICTIM_RECOVERY]`. Since the same address was an unauthorized source, the receipts do not establish independently secure custody.

| UTC | From | Asset | Gross send | Tax | Net receipt | Separate gas, LUNC | Terra transaction |
| --- | --- | --- | --- | --- | --- | --- | --- |

## Accounting files and limits

- [source-chain-transaction-ledger.csv](ledgers/source-chain-transaction-ledger.csv) — 68 selected public source transactions: 66 affected-owner transactions, the relay IBC send, and the return receipt. Consent classifications and network fees are retained.
- [source-chain-asset-flow-ledger.csv](ledgers/source-chain-asset-flow-ledger.csv) — 239 native/CW20 movement edges, including LP redemption, router hops, unwrap burns, treasury payouts, tax, and module fees.
- [bridge-flow-ledger.csv](ledgers/bridge-flow-ledger.csv) — Nine source deposits: gross, fee, net, destination, cross-chain hash, and eight BSC release links.
- [bsc-token-flow-ledger.csv](ledgers/bsc-token-flow-ledger.csv) — Eight verified BSC bridge credits, including the earlier nonce 150.
- [bsc-earlier-swap-ledger.csv](ledgers/bsc-earlier-swap-ledger.csv) — Twenty successful sales, with CL8Y debits, routing fees, actual BNB payouts, WBNB transit, gas, and transaction hashes.
- [bsc-swap-token-routing-ledger.csv](ledgers/bsc-swap-token-routing-ledger.csv) — 388 ERC20 edges from sale receipts, with intermediate token and routing addresses.
- [bsc-native-call-flow-ledger.csv](ledgers/bsc-native-call-flow-ledger.csv) — 128 successful native call edges. Reverted branches and DELEGATECALL apparent values are excluded.
- [bsc-transaction-ledger.csv](ledgers/bsc-transaction-ledger.csv) — All 56 outgoing account nonces, receipt statuses, actions, and exact gas costs.
- [bsc-native-funding-ledger.csv](ledgers/bsc-native-funding-ledger.csv) — Three independently verified incoming BNB funding transactions.
- [osmosis-asset-flow-ledger.csv](ledgers/osmosis-asset-flow-ledger.csv) — Eleven incident-specific Osmosis receive/swap/fee/return edges. Unrelated protorev profit is excluded.

These are **edge ledgers**, not columns to sum as stolen value. The same token appears at successive hops; Terra burns and BSC unlocks represent the same principal. Reconcile original source debits with terminal holdings, confirmed conversions, and identified costs. The export’s USD column is not evidence of realized USD proceeds. A separately identified dated spot valuation of net traced proceeds is recorded in ACCOUNTING.md; acquisition basis and realized USD profit are not established.

The source ledger separates reported unauthorized actions, the user-confirmed earlier Ledger deposit, the reporter-identified normal bot trade, and otherwise-unattributed activity. Interleaved orders are not automatically classified as theft. Four failed source cancellations and six failed BSC swaps remain recorded with their gas costs.

The later-supplied CL8Y export has 39 transfer rows across 26 transactions. Its 33 debit rows across 20 sales sum to 6600 CL8Y. Primary receipts and call trees verify the financial conclusions; explorer method labels and USD markings are not treated as authoritative accounting.

The original investigation was read-only. A later, separately recorded response transaction sent the public recovery memo; its included receipt and timing are recorded in COMMUNICATIONS.md. No protocol contract or production configuration was changed by this archive publication. Independent review verified the earlier bridge source, relevant authorities, all 20 native call trees, all 56 outgoing nonces, three funding receipts, and all 55 native balance closures. Endpoint errors and the initial scope error are retained in restricted evidence.

Full source transaction index:

| UTC | Signer label | Action | Code | Attribution | Terra transaction |
| --- | --- | --- | --- | --- | --- |
| 02:10:35 | [VICTIM_RECOVERY] | send/deposit_cw20_mintable_burn | 0 | User-confirmed unauthorized Ledger-address deposit | [3C04FDDC18D59E52047AD8AAE3BE5C63D4D1F1DCD1C6F4BD7EF58A07EEAA2C0A](https://finder.terraclassic.community/columbus-5/tx/3C04FDDC18D59E52047AD8AAE3BE5C63D4D1F1DCD1C6F4BD7EF58A07EEAA2C0A) |
| 02:22:55 | [VICTIM_B] | cancel_limit_orders | 0 | Unattributed; consent not established | [940677ED915DE4306552D26761E3DCC0F8A13FFC1E8ED9D7B29E646B0C7C191E](https://finder.terraclassic.community/columbus-5/tx/940677ED915DE4306552D26761E3DCC0F8A13FFC1E8ED9D7B29E646B0C7C191E) |
| 02:23:01 | [VICTIM_B] | send/place_limit_order_batch | 0 | Unattributed; consent not established | [66A95611C6D95AF0DF8D44159ED180B35038B12AE9B2B83D996C88DB8E1BC933](https://finder.terraclassic.community/columbus-5/tx/66A95611C6D95AF0DF8D44159ED180B35038B12AE9B2B83D996C88DB8E1BC933) |
| 02:23:07 | [VICTIM_B] | send/place_limit_order_batch | 0 | Unattributed; consent not established | [A44434F2BA557C8940EA44FA6EB29C02851AD492A2E0B2FECA8CBE41EBCDF529](https://finder.terraclassic.community/columbus-5/tx/A44434F2BA557C8940EA44FA6EB29C02851AD492A2E0B2FECA8CBE41EBCDF529) |
| 02:48:33 | [VICTIM_A] | send/execute_swap_operations | 0 | Unattributed; consent not established | [F95FBF14AE3063EBFACC0FA58F43685C94E1A724F3DA3E084891BFBC39844438](https://finder.terraclassic.community/columbus-5/tx/F95FBF14AE3063EBFACC0FA58F43685C94E1A724F3DA3E084891BFBC39844438) |
| 02:50:19 | [VICTIM_B] | send/place_limit_order_batch | 0 | Unattributed; consent not established | [9B368357C8772CFC42C05A27FE566424E6F0FBD7163D3F9A4DFE2FA317E4D98A](https://finder.terraclassic.community/columbus-5/tx/9B368357C8772CFC42C05A27FE566424E6F0FBD7163D3F9A4DFE2FA317E4D98A) |
| 04:19:38 | [VICTIM_A] | send/deposit_cw20_mintable_burn | 0 | Reported unauthorized | [E76AF7A330D76A60EC0557347724E2405B07CF1B4878140EAA722AD400A093E7](https://finder.terraclassic.community/columbus-5/tx/E76AF7A330D76A60EC0557347724E2405B07CF1B4878140EAA722AD400A093E7) |
| 04:43:11 | [VICTIM_B] | send/deposit_cw20_mintable_burn | 0 | Reported unauthorized | [3A5D25F06E769EE61E2E8AC271646A8E0F9F57828102B054F2FB49F635286BAA](https://finder.terraclassic.community/columbus-5/tx/3A5D25F06E769EE61E2E8AC271646A8E0F9F57828102B054F2FB49F635286BAA) |
| 04:43:17 | [VICTIM_B] | send/deposit_cw20_mintable_burn | 0 | Reported unauthorized | [FDDED73075377ADA5C102C76E53C08FBAB331EA934699CC2D2C3B42C852BF1D7](https://finder.terraclassic.community/columbus-5/tx/FDDED73075377ADA5C102C76E53C08FBAB331EA934699CC2D2C3B42C852BF1D7) |
| 04:48:39 | [VICTIM_B] | send/withdraw_liquidity | 0 | Reported unauthorized | [9151D132145AF854941657BC2997DB954AC019CD84C787CC2DA485EF214E8F46](https://finder.terraclassic.community/columbus-5/tx/9151D132145AF854941657BC2997DB954AC019CD84C787CC2DA485EF214E8F46) |
| 04:53:04 | [VICTIM_B] | send/execute_swap_operations | 0 | Reported unauthorized | [4898566E7A3D6CC3245A49E3D0040BC7D3979E149F58BDFC9D8596438407D578](https://finder.terraclassic.community/columbus-5/tx/4898566E7A3D6CC3245A49E3D0040BC7D3979E149F58BDFC9D8596438407D578) |
| 04:53:35 | [VICTIM_B] | cancel_limit_orders | 0 | Unattributed; consent not established | [23A8A6A1BF7FDE0FCEE56BF1A34278055D61F14ED27AE8AE11E67B9E8AE8459A](https://finder.terraclassic.community/columbus-5/tx/23A8A6A1BF7FDE0FCEE56BF1A34278055D61F14ED27AE8AE11E67B9E8AE8459A) |
| 04:53:41 | [VICTIM_B] | send/place_limit_order_batch | 0 | Unattributed; consent not established | [678728D0E2B98709BA8ABC08B299118134982F4E9A751092B2577353671B9059](https://finder.terraclassic.community/columbus-5/tx/678728D0E2B98709BA8ABC08B299118134982F4E9A751092B2577353671B9059) |
| 04:53:47 | [VICTIM_B] | send/execute_swap_operations | 0 | Reported unauthorized | [D2BE2D3C1BC5FF82C229FF8E266AA8DE5F82D2F124AAF56126A216AB8D171435](https://finder.terraclassic.community/columbus-5/tx/D2BE2D3C1BC5FF82C229FF8E266AA8DE5F82D2F124AAF56126A216AB8D171435) |
| 04:54:22 | [VICTIM_B] | cancel_limit_orders | 0 | Unattributed; consent not established | [0743E0916A4D207274495CD21758C46276BB0768445349F6EA798536FB16171D](https://finder.terraclassic.community/columbus-5/tx/0743E0916A4D207274495CD21758C46276BB0768445349F6EA798536FB16171D) |
| 04:54:28 | [VICTIM_B] | send/place_limit_order_batch | 0 | Unattributed; consent not established | [C9E8F2C39B07FAAA39436E996D40E225BD69BE3214D4ED6596D8585E440F7755](https://finder.terraclassic.community/columbus-5/tx/C9E8F2C39B07FAAA39436E996D40E225BD69BE3214D4ED6596D8585E440F7755) |
| 04:57:14 | [VICTIM_B] | send/execute_swap_operations | 0 | Reported unauthorized | [CEDC3D29AAB84E9FBE6FA98587FAC54A93E677650956AEEBB2884DFB45D875C9](https://finder.terraclassic.community/columbus-5/tx/CEDC3D29AAB84E9FBE6FA98587FAC54A93E677650956AEEBB2884DFB45D875C9) |
| 05:00:53 | [VICTIM_B] | send/withdraw_liquidity | 0 | Reported unauthorized | [E0C7D739846CDC1A2482C40227242D111BAD7177E9D7E63C500213266D9BA640](https://finder.terraclassic.community/columbus-5/tx/E0C7D739846CDC1A2482C40227242D111BAD7177E9D7E63C500213266D9BA640) |
| 05:00:58 | [VICTIM_B] | send/unwrap | 0 | Reported unauthorized | [9BAB7E8323A12DDB6F59274C54D05E3FA08926CCA7D2F7481CE90D55826E87DB](https://finder.terraclassic.community/columbus-5/tx/9BAB7E8323A12DDB6F59274C54D05E3FA08926CCA7D2F7481CE90D55826E87DB) |
| 05:04:02 | [VICTIM_A] | send/execute_swap_operations | 0 | Reporter identifies as bot trade | [27799A1B220324ACA0750818CA144E9E0203E907690799A3835B01E908F18618](https://finder.terraclassic.community/columbus-5/tx/27799A1B220324ACA0750818CA144E9E0203E907690799A3835B01E908F18618) |
| 05:04:08 | [VICTIM_B] | cancel_limit_orders | 0 | Unattributed; consent not established | [47B3787A9A53012BC206A40F8AE6349F5AA49FD94E935365C57552326F0D2643](https://finder.terraclassic.community/columbus-5/tx/47B3787A9A53012BC206A40F8AE6349F5AA49FD94E935365C57552326F0D2643) |
| 05:04:14 | [VICTIM_B] | send/place_limit_order_batch | 0 | Unattributed; consent not established | [158977DD6FAC26DAE6C127A26294A472959708405A19083224808AE2D513B9BB](https://finder.terraclassic.community/columbus-5/tx/158977DD6FAC26DAE6C127A26294A472959708405A19083224808AE2D513B9BB) |
| 05:04:20 | [VICTIM_B] | cancel_limit_order | 5 | Unattributed; consent not established | [4C82F6B796B7332BA98FA8912A9DC1A56641A1A762942EF7E906CCC0D2873319](https://finder.terraclassic.community/columbus-5/tx/4C82F6B796B7332BA98FA8912A9DC1A56641A1A762942EF7E906CCC0D2873319) |
| 05:04:25 | [VICTIM_B] | cancel_limit_order | 5 | Unattributed; consent not established | [86F9B0766D3B354E5462B5474FE1E051628FFC7DE1920BA5E80AEA230A35DED5](https://finder.terraclassic.community/columbus-5/tx/86F9B0766D3B354E5462B5474FE1E051628FFC7DE1920BA5E80AEA230A35DED5) |
| 05:04:43 | [VICTIM_B] | cancel_limit_order | 0 | Unattributed; consent not established | [C8DA0DF1894E7436EA8F708EE25492E56413BE585115840FCAE55057E3E68A17](https://finder.terraclassic.community/columbus-5/tx/C8DA0DF1894E7436EA8F708EE25492E56413BE585115840FCAE55057E3E68A17) |
| 05:05:01 | [VICTIM_B] | cancel_limit_order | 0 | Unattributed; consent not established | [D6D20CDDA4F7E175375AAAC60C1C70B3227FB4F7F2D67A63B54829B208AD82E7](https://finder.terraclassic.community/columbus-5/tx/D6D20CDDA4F7E175375AAAC60C1C70B3227FB4F7F2D67A63B54829B208AD82E7) |
| 05:05:13 | [VICTIM_B] | cancel_limit_order | 0 | Unattributed; consent not established | [DFF6A9A29132C310E21E951C6753AE65DC1057AD69551D68C07BF627787BC028](https://finder.terraclassic.community/columbus-5/tx/DFF6A9A29132C310E21E951C6753AE65DC1057AD69551D68C07BF627787BC028) |
| 05:05:43 | [VICTIM_B] | send/place_limit_order_batch | 0 | Unattributed; consent not established | [33AC44115B90C31273FA391A35E15CD82B4B7F8C64BEF6615896E0858DEB90C0](https://finder.terraclassic.community/columbus-5/tx/33AC44115B90C31273FA391A35E15CD82B4B7F8C64BEF6615896E0858DEB90C0) |
| 05:05:54 | [VICTIM_B] | cancel_limit_order | 5 | Unattributed; consent not established | [C48A176E9E5BF331A47913A202DFD298BE0EA708920A8A08DF31B9FDDAEB0BE0](https://finder.terraclassic.community/columbus-5/tx/C48A176E9E5BF331A47913A202DFD298BE0EA708920A8A08DF31B9FDDAEB0BE0) |
| 05:06:24 | [VICTIM_B] | cancel_limit_order | 5 | Unattributed; consent not established | [59D3A44B098A329B33E7C406446B5D2B1AA3D1FB1B778C1A3851ABEE7CA27ADB](https://finder.terraclassic.community/columbus-5/tx/59D3A44B098A329B33E7C406446B5D2B1AA3D1FB1B778C1A3851ABEE7CA27ADB) |
| 05:07:17 | [VICTIM_B] | cancel_limit_order | 0 | Unattributed; consent not established | [E588234161F66BF760951B8E6F34960822A4388C530E82FDDAC256E77761EBFB](https://finder.terraclassic.community/columbus-5/tx/E588234161F66BF760951B8E6F34960822A4388C530E82FDDAC256E77761EBFB) |
| 05:07:23 | [VICTIM_B] | send/place_limit_order_batch | 0 | Unattributed; consent not established | [F68A646670219A9549CD759FB5FE3BD32A77A7C87B03D73BE744707D9C7B7148](https://finder.terraclassic.community/columbus-5/tx/F68A646670219A9549CD759FB5FE3BD32A77A7C87B03D73BE744707D9C7B7148) |
| 05:08:04 | [VICTIM_B] | cancel_limit_order | 0 | Unattributed; consent not established | [170F6170817B31035E84FE27C215030C88F900E6F690FBB2F0562CDFD492E895](https://finder.terraclassic.community/columbus-5/tx/170F6170817B31035E84FE27C215030C88F900E6F690FBB2F0562CDFD492E895) |
| 05:08:16 | [VICTIM_B] | cancel_limit_order | 0 | Unattributed; consent not established | [3F2715423A2551AD48DF4F537AE83FD082554784204946170DE2AB0A5C03A98F](https://finder.terraclassic.community/columbus-5/tx/3F2715423A2551AD48DF4F537AE83FD082554784204946170DE2AB0A5C03A98F) |
| 05:08:28 | [VICTIM_B] | send/place_limit_order_batch | 0 | Unattributed; consent not established | [05F9D6E7218270F0E911C926CF97C6180589D73E0480B79AC7158089246D1AFA](https://finder.terraclassic.community/columbus-5/tx/05F9D6E7218270F0E911C926CF97C6180589D73E0480B79AC7158089246D1AFA) |
| 05:10:26 | [VICTIM_B] | cancel_limit_orders | 0 | Unattributed; consent not established | [2B24384A45BCB341A88C6C6009665EB71DE833C077234508D7E4E4663EA13657](https://finder.terraclassic.community/columbus-5/tx/2B24384A45BCB341A88C6C6009665EB71DE833C077234508D7E4E4663EA13657) |
| 05:10:32 | [VICTIM_B] | send/place_limit_order_batch | 0 | Unattributed; consent not established | [9EA2FB53F31F0AF70C597A15253B6ECEABFB9EBC6DA7BA2422F2CD7341D011CB](https://finder.terraclassic.community/columbus-5/tx/9EA2FB53F31F0AF70C597A15253B6ECEABFB9EBC6DA7BA2422F2CD7341D011CB) |
| 05:15:15 | [VICTIM_B] | cancel_limit_order | 0 | Unattributed; consent not established | [5122FE9DEAD17C46C70647288A0231B8985E606D69CB32496D228DB86B4D0A0D](https://finder.terraclassic.community/columbus-5/tx/5122FE9DEAD17C46C70647288A0231B8985E606D69CB32496D228DB86B4D0A0D) |
| 05:15:21 | [VICTIM_B] | cancel_limit_order | 0 | Unattributed; consent not established | [82CCD84B1E220A9955AB388B4A83714936A09F253736B47BDFDB862B2D8D5F86](https://finder.terraclassic.community/columbus-5/tx/82CCD84B1E220A9955AB388B4A83714936A09F253736B47BDFDB862B2D8D5F86) |
| 05:15:33 | [VICTIM_B] | cancel_limit_order | 0 | Unattributed; consent not established | [E4BDE4DF84738015ADE0ED87AA983D7E04BF8E0F5B4D9A5E19FCB5B17D6F0D81](https://finder.terraclassic.community/columbus-5/tx/E4BDE4DF84738015ADE0ED87AA983D7E04BF8E0F5B4D9A5E19FCB5B17D6F0D81) |
| 05:15:45 | [VICTIM_B] | send/place_limit_order_batch | 0 | Unattributed; consent not established | [2D3E38811B64DFA3C3ADE963EA39E4E425DF93761B5504EA30EE6F2224D78323](https://finder.terraclassic.community/columbus-5/tx/2D3E38811B64DFA3C3ADE963EA39E4E425DF93761B5504EA30EE6F2224D78323) |
| 05:15:51 | [VICTIM_B] | cancel_limit_order | 0 | Unattributed; consent not established | [4C6D73697D7137344B705DBB91464E6ADFC7C7BB1A960127C89138F0DF4B255E](https://finder.terraclassic.community/columbus-5/tx/4C6D73697D7137344B705DBB91464E6ADFC7C7BB1A960127C89138F0DF4B255E) |
| 05:19:41 | [VICTIM_B] | cancel_limit_orders | 0 | Unattributed; consent not established | [41EBCF28FC325C818D3CCBAE2CD78D6608BFB6D54DFDFBC821ED5A028384EB92](https://finder.terraclassic.community/columbus-5/tx/41EBCF28FC325C818D3CCBAE2CD78D6608BFB6D54DFDFBC821ED5A028384EB92) |
| 05:19:52 | [VICTIM_B] | send/place_limit_order_batch | 0 | Unattributed; consent not established | [5C93B1DC34984575151EBAFF6903F737EFC23ED1DF1C2FA1FFBE39C91E8513AD](https://finder.terraclassic.community/columbus-5/tx/5C93B1DC34984575151EBAFF6903F737EFC23ED1DF1C2FA1FFBE39C91E8513AD) |
| 05:20:10 | [VICTIM_B] | cancel_limit_orders | 0 | Unattributed; consent not established | [86B2EACEF9021838DF3E8D63C9F038398FFD9E45EFFF6F8FBBD0745B99F042A1](https://finder.terraclassic.community/columbus-5/tx/86B2EACEF9021838DF3E8D63C9F038398FFD9E45EFFF6F8FBBD0745B99F042A1) |
| 05:20:15 | [VICTIM_B] | cancel_limit_orders | 0 | Unattributed; consent not established | [871D937F6A37B8D43C7FC5628BED6CDFA2AB6454085B900C9060C36DCADC3C22](https://finder.terraclassic.community/columbus-5/tx/871D937F6A37B8D43C7FC5628BED6CDFA2AB6454085B900C9060C36DCADC3C22) |
| 05:20:22 | [VICTIM_B] | cancel_limit_orders | 0 | Unattributed; consent not established | [12EFD5E4C3C4F21A05CE7E1B85F35AEC86580C304BB5567B8C0F8FB4EF8F2138](https://finder.terraclassic.community/columbus-5/tx/12EFD5E4C3C4F21A05CE7E1B85F35AEC86580C304BB5567B8C0F8FB4EF8F2138) |
| 05:20:34 | [VICTIM_B] | cancel_limit_orders | 0 | Unattributed; consent not established | [E7F3248FF520912BA1A47645E16A78549711A20ED26249F0618C32F7D5A3AA5D](https://finder.terraclassic.community/columbus-5/tx/E7F3248FF520912BA1A47645E16A78549711A20ED26249F0618C32F7D5A3AA5D) |
| 05:20:39 | [VICTIM_B] | send/place_limit_order_batch | 0 | Unattributed; consent not established | [6E3F4BA88D09F89A09D0C09D0C15C8AEA9D34AD5C81854B0CE2040776DAE0BAF](https://finder.terraclassic.community/columbus-5/tx/6E3F4BA88D09F89A09D0C09D0C15C8AEA9D34AD5C81854B0CE2040776DAE0BAF) |
| 05:23:19 | [VICTIM_B] | cancel_limit_orders | 0 | Unattributed; consent not established | [243E8E484651155D0DE7897A281E2F6656E8B0961FDDE530C95278A1C54EFFD7](https://finder.terraclassic.community/columbus-5/tx/243E8E484651155D0DE7897A281E2F6656E8B0961FDDE530C95278A1C54EFFD7) |
| 05:23:25 | [VICTIM_B] | send/place_limit_order_batch | 0 | Unattributed; consent not established | [73968668A080DAA38693A2BF094E3E45BB5E5A7513A4074D94670339F734B7FA](https://finder.terraclassic.community/columbus-5/tx/73968668A080DAA38693A2BF094E3E45BB5E5A7513A4074D94670339F734B7FA) |
| 05:30:49 | [VICTIM_B] | cancel_limit_orders | 0 | Unattributed; consent not established | [3BF6C0D2A75146AA884D5ADC70CB512A205E3EB74599DD03231AC7F7728FF5BF](https://finder.terraclassic.community/columbus-5/tx/3BF6C0D2A75146AA884D5ADC70CB512A205E3EB74599DD03231AC7F7728FF5BF) |
| 05:30:55 | [VICTIM_B] | send/place_limit_order_batch | 0 | Unattributed; consent not established | [B5CB3E6FF658FA221C07A11087A051C2E552BF21D6F383064027123E2F5A66A3](https://finder.terraclassic.community/columbus-5/tx/B5CB3E6FF658FA221C07A11087A051C2E552BF21D6F383064027123E2F5A66A3) |
| 05:31:30 | [VICTIM_B] | cancel_limit_orders | 0 | Unattributed; consent not established | [FFFED7F62934660AB5B0E6C93CA641EF8D4B6633D896CE6D4DBDBBE26D83805A](https://finder.terraclassic.community/columbus-5/tx/FFFED7F62934660AB5B0E6C93CA641EF8D4B6633D896CE6D4DBDBBE26D83805A) |
| 05:31:36 | [VICTIM_B] | send/place_limit_order_batch | 0 | Unattributed; consent not established | [8A2AF615AA8C9324A291C99EF923B84EDE717D74D4EE31C503B39CC776573924](https://finder.terraclassic.community/columbus-5/tx/8A2AF615AA8C9324A291C99EF923B84EDE717D74D4EE31C503B39CC776573924) |
| 05:32:30 | [VICTIM_A] | send/unwrap | 0 | Reported unauthorized | [16B4C16E1417B180C1389459BC9D9C86E5E63CE98AC5DCCA946CF5282C8110B7](https://finder.terraclassic.community/columbus-5/tx/16B4C16E1417B180C1389459BC9D9C86E5E63CE98AC5DCCA946CF5282C8110B7) |
| 05:32:42 | [VICTIM_A] | send/unwrap | 0 | Unattributed; consent not established | [B2AF113F931203D431E755ED53CE8095F9FB92B021F5E91BF9185D55F0BB3381](https://finder.terraclassic.community/columbus-5/tx/B2AF113F931203D431E755ED53CE8095F9FB92B021F5E91BF9185D55F0BB3381) |
| 05:33:23 | [VICTIM_B] | send/unwrap | 0 | Reported unauthorized | [A79920E3A17A8EAF78F9B5959BE8CCD57832E23AAE86BC6CA7C6E58C7CED062B](https://finder.terraclassic.community/columbus-5/tx/A79920E3A17A8EAF78F9B5959BE8CCD57832E23AAE86BC6CA7C6E58C7CED062B) |
| 05:35:27 | [VICTIM_A] | MsgSend | 0 | Reported unauthorized | [88D1A861F775079ED7DEACD31028EB5B2B25E2FF6DCD9E8EE7E9554D70C4D9DB](https://finder.terraclassic.community/columbus-5/tx/88D1A861F775079ED7DEACD31028EB5B2B25E2FF6DCD9E8EE7E9554D70C4D9DB) |
| 05:35:45 | [VICTIM_A] | MsgSend | 0 | Reported unauthorized | [D9BE15AE716C1977F822CBA2C5947C181AFAFF25FEE2DE4A586D96453E475FE2](https://finder.terraclassic.community/columbus-5/tx/D9BE15AE716C1977F822CBA2C5947C181AFAFF25FEE2DE4A586D96453E475FE2) |
| 05:36:03 | [VICTIM_B] | MsgSend | 0 | Reported unauthorized | [2C5F233DA80C42B0A1614F3B9A6BBE1E3083E4EE73452F955DDCEB109BAF4BC4](https://finder.terraclassic.community/columbus-5/tx/2C5F233DA80C42B0A1614F3B9A6BBE1E3083E4EE73452F955DDCEB109BAF4BC4) |
| 05:47:46 | [VICTIM_A] | send/deposit_cw20_mintable_burn | 0 | Reported unauthorized | [B350B9AE464FA22EED2E17A743C177E72B0BF658F859DE5654D0BA9CFE3472F6](https://finder.terraclassic.community/columbus-5/tx/B350B9AE464FA22EED2E17A743C177E72B0BF658F859DE5654D0BA9CFE3472F6) |
| 05:53:41 | [VICTIM_A] | send/deposit_cw20_mintable_burn | 0 | Reported unauthorized | [5AAF95217E21E94BC7089750EA00EE5AC32E0E39D1F22AC95E61EB5A39C18824](https://finder.terraclassic.community/columbus-5/tx/5AAF95217E21E94BC7089750EA00EE5AC32E0E39D1F22AC95E61EB5A39C18824) |
| 05:54:47 | [VICTIM_A] | send/deposit_cw20_mintable_burn | 0 | Reported unauthorized | [E004EEE6DBEA8DFEB0E45C9766E002F6E836F3FBA47B66086B4CC21E52E31132](https://finder.terraclassic.community/columbus-5/tx/E004EEE6DBEA8DFEB0E45C9766E002F6E836F3FBA47B66086B4CC21E52E31132) |
| 06:01:29 | [VICTIM_B] | send/deposit_cw20_mintable_burn | 0 | Reported unauthorized | [A418EABA2E5C50A19938791F37FD414F1416EA204C535B46B157E63DED9637E4](https://finder.terraclassic.community/columbus-5/tx/A418EABA2E5C50A19938791F37FD414F1416EA204C535B46B157E63DED9637E4) |
| 06:01:59 | [VICTIM_B] | send/deposit_cw20_mintable_burn | 0 | Reported unauthorized | [4334416A150C402ED31DC435A58F8DDB2E227372FA8CE54BD7138A0444B608B9](https://finder.terraclassic.community/columbus-5/tx/4334416A150C402ED31DC435A58F8DDB2E227372FA8CE54BD7138A0444B608B9) |
| 06:02:46 | [HACKER_C] | MsgTransfer | 0 | Reported unauthorized | [58EBD8FD50CB9906DE6C90F05C08FCF72B6175316D4A9DCEC8AC22E1740488DA](https://finder.terraclassic.community/columbus-5/tx/58EBD8FD50CB9906DE6C90F05C08FCF72B6175316D4A9DCEC8AC22E1740488DA) |
| 06:03:27 | [IBC_RELAYER] | MsgUpdateClient; MsgRecvPacket; MsgAcknowledgement | 0 | Confirmed return of traced IBC funds | [339289860E63E23D93F7C89E7A3D8B2DE37DCBE9DC2C8DF01EB3620428B497E4](https://finder.terraclassic.community/columbus-5/tx/339289860E63E23D93F7C89E7A3D8B2DE37DCBE9DC2C8DF01EB3620428B497E4) |

Financial coverage ends 2026-10-15 09:12:20 UTC. The original containment snapshot was 08:54 UTC. The review does not provide ongoing monitoring or establish that a control remained unchanged after its stated snapshot.

Updated 2026-10-15 09:43 UTC.
