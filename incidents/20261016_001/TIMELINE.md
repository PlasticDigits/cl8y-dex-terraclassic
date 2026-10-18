# Timeline and scope corrections

The original incident timeline below uses UTC on 2026-10-15. A separate 16 October response timeline follows. Transaction-level source and destination indexes, including routine/unattributed actions and failed attempts, are retained in [ledgers/](ledgers/). Actor labels identify addresses, not proven real-world controllers.

| Time | Event | Evidence / accounting implication |
| --- | --- | --- |
| 00:00 | Expanded BSC baseline | `[HACKER_A]` native balance and account nonce were zero |
| 02:10:35 | Ledger-labelled `[VICTIM_RECOVERY]` deposits CL8Y, source nonce 150 | Gross 8271.070656475910303881; net 8246.257444506482572970 CL8Y |
| 02:13:08 | First small native funding credit to `[HACKER_A]` | One of three funding receipts; controller un-attributed |
| 02:18:21 | Nonce 150 unlock executes on BSC | Earlier CL8Y release absent from the original supplied reports |
| 02:28:45–02:49:56 | Twenty completed CL8Y sales | Total debit 6600 CL8Y; actual native credits 3.943439176513535698 BNB |
| 04:00 | Initial investigation's BSC cutoff | Missed the earlier release and sales; 1646.257444506482572970 CL8Y appeared as opening inventory |
| 04:19:38 | Source USDT deposit 152 | Net 236.653934613162707760 USDT |
| 04:25:04 | BSC release 152 | USDT reaches `[HACKER_A]` |
| 04:43:11 | Source vFDUSD deposit 153 | Net 10.154338; destination withdrawal unsubmitted at checked hash |
| 04:43:17 | Source USDT deposit 154 | Net 341.682420857001980921 USDT |
| 04:48:34 | BSC release 154 | Total traced USDT receipts reach 578.336355470164688681 |
| 05:35:27–05:36:03 | Native USTC/LUNC payments to `[HACKER_C]` | Gross/native-tax/net figures in ACCOUNTING.md |
| 05:47:46–06:01:59 | Source CL8Y deposits 155–159 | Includes one-wei test; five net credits total 23806.246503847620234459 CL8Y |
| 05:53:05–06:07:16 | BSC releases 155–159 | Together with earlier residue, CL8Y reaches 25452.503948354102807429 |
| 06:02:46 | Relay sends USTC through IBC | Packet 395425, Terra channel-1 → Osmosis channel-72 |
| 06:03:17 | Osmosis receives, swaps and initiates return | Pool 562; exact input/output/affiliate splits captured |
| 06:03:27 | Return packet executes on Terra | Packet 583917 credits 4324206.487715 native LUNC to `[HACKER_C]` |
| 06:06:59 | Later approval of 1000 CL8Y | Allowance change; distinct from the 20 earlier sales |
| 08:54 snapshot | Dated control/balance observations | CL8Y sending restrictions and BSC pause observed |
| 09:12:20 | Complete recorded BSC flow coverage ends | Includes successful tail scan with no additional token flows in that tail |
| 09:44:22 | Later BSC spot check | Native/token balances unchanged; account nonce 56; not an additional complete log scan |
| 10:24:17–10:24:35 | USD quote observations | Dated spot valuation of net non-CL8Y proceeds, excluding frozen CL8Y |
| 10:42:06 | Public Terra recovery notice included | Separately funded response; outside the theft-accounting cutoff |

## Later reconstructed movements

The original table retains its original window. These receipts extend the chronology; all event dates remain 15 October 2026 UTC.

| UTC | Event | Evidence |
| --- | --- | --- |
| 13:41:29 | [FUNDING_KUCOIN_28] → [HACKER_A]; 0.613347 BNB | [0x26b098e4a5b96d53563c901b38424705b5bfbd1bfb5ae7a5a26563f3484aaf44](https://bscscan.com/tx/0x26b098e4a5b96d53563c901b38424705b5bfbd1bfb5ae7a5a26563f3484aaf44) |
| 13:45:48 | [HACKER_A] → [SERVICE_OR_CONTRACT]; 2.81 BNB | [0xeec21b74576c017c504aadfd37112856fdd54e6c6a7db47bcdc84a94c65b38bc](https://bscscan.com/tx/0xeec21b74576c017c504aadfd37112856fdd54e6c6a7db47bcdc84a94c65b38bc) |
| 13:45:59 | [SERVICE_OR_CONTRACT] → [HACKER_A]; 0.853034552479184202 ETH | [0xa18414194daf282989e360d41a20cd593e5688ca347f265ebd71a98c2282776c](https://etherscan.io/tx/0xa18414194daf282989e360d41a20cd593e5688ca347f265ebd71a98c2282776c) |
| 13:46:35 | [HACKER_A] → [ETH_FORWARDER_A]; 0.852 ETH | [0xd7340ff0c82a79d29b6fbea7b7dabaec4cbd49eeea367b8bc33245945540a4dc](https://etherscan.io/tx/0xd7340ff0c82a79d29b6fbea7b7dabaec4cbd49eeea367b8bc33245945540a4dc) |
| 14:10:23 | [HACKER_A] → [BNB_FORWARDER_A]; 2.54 BNB | [0xd956503b88fe9a54ee8c558262b25877c61c501b897c3ccab45486ac659ccf86](https://bscscan.com/tx/0xd956503b88fe9a54ee8c558262b25877c61c501b897c3ccab45486ac659ccf86) |
| 14:11:25 | [BNB_FORWARDER_A] → [EXCHANGE_FIXEDFLOAT]; 2.53999853 BNB | [0x84547676907c2806934d49ff2e9d9eb20c340430de1feee6fe3305ae32bda7f2](https://bscscan.com/tx/0x84547676907c2806934d49ff2e9d9eb20c340430de1feee6fe3305ae32bda7f2) |

13:42:59 UTC: 578.336355470164688681 USDT converted through 1inch into 0.802777871826063680 BNB. Ethereum split-transfer times and hashes are tabulated in [BNB_FOLLOWUP.md](BNB_FOLLOWUP.md). Later balances were captured on 16 October; this is not a new complete all-token log census.

## How the investigation changed

1. The initial Markdown report covered three bridge deposits. The later PDF added CL8Y transfers and native transfers but still omitted nonce 150 and the earlier sales.
2. The first independent BSC review started at 04:00 UTC. Its no-outgoing-token finding was correct within that window but did not establish the absence of earlier sales.
3. A supplied CL8Y export contained 39 transfer rows across 26 transactions, including the 02:18 release and 20 earlier sales. Their 33 debit rows total 6600 CL8Y, including routing fees.
4. Primary receipts, expanded logs, all 56 outgoing account transactions, full native call trees, and three funding receipts resolved the earlier period. The source of nonce 150 matched the address previously described as the hardware recovery wallet.
5. The requester relayed the victim's confirmation that the earlier deposit and sales were unauthorized, and clarified that this wallet was a Ledger wallet while the other two were software wallets.

The source narratives remain evidence of what was originally reported, rather than substitutes for the corrected chain findings. The record does not infer a cause for the Ledger-address signature from that custody description.

## Completed bridge response — 16 October 2026 UTC

| Time | Event | Evidence / interpretation |
| --- | --- | --- |
| 02:27:57 | BSC assessment balance snapshot | CL8Y unchanged at the first BSC attacker wallet; USDT zero and BNB 0.003397872962218485; later balances do not trace disposition |
| 02:37:21 | Vault pays BSC recovery wallet | 10.154338 vFDUSD net; source fee not reimbursed; original nonce 153 not consumed |
| 02:42:36–02:42:43 | Deploy blacklist, mark both BSC attackers, install withdrawal and deposit modules | Four successful control transactions; BSC remains paused |
| 02:44:55 | Pre-unpause verification snapshot | Both attacker wallets rejected by full-guard simulations; recovery wallet allowed; nonce 153 still unused |
| 02:45:37 | BSC bridge unpaused | Successful transaction at block 127900807 |
| 02:47:48 | Pinned post-unpause state | BSC unpaused, both blacklist modules installed, recovery balance 10.154338 vFDUSD, original claim unsubmitted and nonce unused |

Exact transaction links, public captures and scope limits: [BRIDGE_RECOVERY.md](BRIDGE_RECOVERY.md). The historical 15 October theft ledger is retained unchanged.

## Subsequently reported exchange outreach

Recorded on 16 October 2026 UTC: the requester confirmed both prepared FixedFloat and KuCoin emails were sent from `contact@cl8y.com`. The exact sending times were not provided, so these communications are not assigned invented timestamps or ordered against the independently included bridge-response transactions. See [COMMUNICATIONS.md](COMMUNICATIONS.md) and [email copies/provenance](recovery/EXCHANGE_EMAILS_20261016.md). No delivery acknowledgement or exchange response is established.

## NFT recovery and reserve compensation — 17–18 October 2026 UTC

| UTC | Event | Evidence / interpretation |
| --- | --- | --- |
| 17 October 00:27:56 | Penguin #1509 transferred from the original affected address to the designated Terra recovery address | Successful transaction at height 30870523; later recipient ownership also verified |
| 18 October 07:38:36 | Project reserve transfers 25,452.503948354102807429 CL8Y to the designated BSC recovery wallet | Successful transaction at BSC block 128323680; reserve compensation, not attacker-token seizure |
| 18 October approximately 07:41:54 | Completed NFT recovery ownership check | Recipient ownership of #1509 confirmed at Terra height 30889465 by Hexxagon; PublicNode agrees without response-height metadata |
| 18 October 07:42:46 | BSC custody snapshot | Block 128324235: recovery wallet holds compensation CL8Y and 10.154338 vFDUSD; attacker CL8Y unchanged and sender-blacklist entry present |

Receipts, query provenance and limitations are in [RECOVERY_UPDATE_20261018.md](RECOVERY_UPDATE_20261018.md). The separately supplied messages have 17–18 October dates but no stated timezone, so their clock times are not interleaved with these UTC events. Police reporting remains a participant statement.
