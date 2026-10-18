# Ledger dictionary

UTF-8 CSV files use header rows, decimal strings and integer base units. `raw_*` / `*_raw` / `*_wei` fields are exact integer quantities. Human-readable `amount` fields apply the stated decimals; rounding in narrative USD tables is separate from raw accounting.

| File | Rows | Grain and scope |
| --- | ---: | --- |
| `source-chain-transaction-ledger.csv` | 68 | One captured Terra transaction, including relay send/return; status, signer label/address, action, consent attribution and declared network fee |
| `source-chain-asset-flow-ledger.csv` | 239 | One native transfer or CW20 send/transfer/mint/burn event edge; successive hops and module distributions are distinct edges |
| `bridge-flow-ledger.csv` | 9 | One source deposit; gross, fee, net, destination, cross-chain ID and optional BSC execution |
| `bsc-token-flow-ledger.csv` | 8 | One actual BSC bridge release token credit |
| `bsc-earlier-swap-ledger.csv` | 20 | One successful CL8Y sale; account nonce, debit/fee, actual BNB credit, WBNB transit, gas and routing symbols |
| `bsc-swap-token-routing-ledger.csv` | 388 | One ERC20 `Transfer` event from a successful sale receipt, including intermediate pools/routers |
| `bsc-native-call-flow-ledger.csv` | 128 | One successful positive native-value call edge from the sale call trees |
| `bsc-transaction-ledger.csv` | 56 | One attacker-originated BSC transaction, all nonces0–55, including six failures and22 approvals |
| `bsc-native-funding-ledger.csv` | 3 | One independent direct native funding transfer; source ownership un-attributed |
| `osmosis-asset-flow-ledger.csv` | 11 | One transfer edge in the incident-related receive/swap/fee/return message; unrelated module activity excluded |
| `bridge-response-transactions.csv` | 6 | One successful BSC response transaction on 16 October: vault payout, four blacklist operations, unpause; separate from theft principal and attacker gas |

## Common identifiers

- `tx_hash` / source/destination transaction fields identify public-chain transactions. Explorer links appear in the report; raw captures are grouped by hash under `evidence/`.
- `block_height`, `bsc_block`, account nonce, event/log index and native call path preserve ordering and capture relationships. A call path such as `0.4` identifies a child in the preserved call tree.
- Wallet labels map to addresses in [REPORT.md](REPORT.md) and [evidence/labels.json](evidence/labels.json). The original `A`/`B` software-wallet labels map to `[VICTIM_A]` / `[VICTIM_B]`; `relay` maps to `[HACKER_C]`. The later Ledger source retains the historical `[VICTIM_RECOVERY]` label despite being affected.
- `[PROTOCOL_OR_RELAYER]` and similar generic labels do not establish a person's ownership. Complete public addresses remain in the edge columns.

## Units and fees

- `uluna` is LUNC base units, six decimals. `uusd` is USTC base units, six decimals.
- The relevant BSC CL8Y, USDT and WBNB quantities use18 decimals. The source vFDUSD amount uses six decimals; the BSC vFDUSD recovery uses eight decimals (10154338 Terra units = 1015433800 BSC units = 10.154338 vFDUSD). Contract/denomination fields disambiguate similarly named assets on different chains.
- Source `network_fee_LUNC` is derived from the transaction's declared `auth_info.fee` amount. It is not computed by multiplying reported gas used by a price; the transfer-edge ledger separately preserves actual emitted movements.
- BSC `gas_paid_wei` is receipt `gasUsed × effectiveGasPrice`. Gas is counted for successful and failed outgoing transactions. Native transaction value is a separate field and is zero across all56 captured outgoing transactions.
- Bridge gross/net/fee fields use source units. The burn and matching destination unlock are two representations of one transfer, not two receipts to add together.
- The16.5CL8Y routing fee is included in the6600CL8Y sale debit. Some fee edges are direct from the wallet and some pass through execution contracts.
- WBNB unwrapped is a route observation. Actual BNB wallet credits are determined from native call trees and corroborated by block balances. Those measures are not added as independent gains.

## Consent, success and attribution

`User-confirmed unauthorized Ledger-address deposit` records the requester's relay of the victim's denial of consent for the earlier source deposit and sales. Other `Reported unauthorized` classifications trace to the supplied reports. `Reporter identifies as bot trade` and otherwise-unattributed actions remain distinct.

Code0 on Terra and status1 on BSC indicate included success in the captured responses. Failure rows remain in transaction/fee accounting and do not become completed principal transfers. Chain inclusion or a valid signature does not independently establish consent, intent, device custody or the identity of a controller.

## Follow-up transaction ledger

`followup-transactions.csv` contains 17 rows: eight BSC transactions (six source outgoing nonces 56–61, one outside funding payment and one forwarder transfer), plus nine Ethereum transactions (Relay fill, source nonce 0 payment, and seven forwarder payments). Native quantities use 18 decimals. `value_wei` is the outer transaction value, not swap output; the swap's actual BNB credit is in accounting-summary.json and its call tree. `gas_fee_wei` is receipt gas used × effective gas price and belongs only to `sender`. Labels describe observed addresses, not independently identified people. Contract/service labels do not establish culpability. Follow-up accounting is documented separately; no new automated financial assertions were added or run.
