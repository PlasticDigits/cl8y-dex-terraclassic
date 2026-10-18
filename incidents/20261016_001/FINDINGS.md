# Findings and uncertainty

The [16 October continuation](BNB_FOLLOWUP.md) adds verified later service-bound BNB transfers, USDT conversion and an Ethereum bridge continuation. Earlier balances/no-onward-flow observations remain explicitly historical. Exchange customer identity and final off-ramp outputs remain unestablished.

The [17–18 October recovery follow-up](RECOVERY_UPDATE_20261018.md) establishes reserve-funded CL8Y compensation and Penguin #1509 receipt. Police reporting is participant-reported. No new finding about the signing-compromise mechanism follows from the anonymized messages.

## Supported observations

| Finding | Basis | Interpretation limit |
| --- | --- | --- |
| Unauthorized use of three source signing authorities | Valid included transactions, account/public-key agreement, victim's denial of consent | Chain data proves accepted signing authority, not the mechanism or person producing the signature |
| Earlier CL8Y liquidation occurred | 20 successful receipts and native call trees; export reconciles 6600 CL8Y debit | Router use and intermediary tokens alone do not establish laundering intent |
| Reported Ledger recovery address was also an unauthorized source | Source nonce 150 and victim clarification | The device type is victim-reported; device/firmware compromise or seed extraction is not established |
| Bridge payouts matched source deposits | Cross-chain identifiers, source burns/fees, destination receipt/getter agreement | Correct source-chain data can correspond to a transaction the wallet owner did not consent to |
| No reviewed contract takeover or exploit identified | Dated role/config/code-history observations and normal funded execution traces | Not a full code audit, host investigation, consensus proof or exhaustive historical-permission audit |
| CL8Y sending restriction and BSC pause were observed | Read-only getters and transfer/guard simulations | Snapshot-specific; asset-specific restrictions do not describe all assets or all later times |
| Deployed guard has no working CL8Y forced-transfer path | Live guard/allowance/blacklist reads and the reproduced `ERC20InsufficientAllowance` revert | Assessment of the reviewed guard path; not a fix, and no asset was moved |
| Unsubmitted vFDUSD claim remained at the original snapshot | Getter absent at checked hash, source nonce unused | Absence is not cancellation or permanent revocation |
| Net vFDUSD principal recovered on 16 October | Successful vault payout and exact token transfer to the newly designated BSC recovery address | Source fees were not reimbursed; original nonce 153 was not consumed |
| CL8Y reserve compensation completed on 18 October | Successful receipt and exact 25,452.503948354102807429 CL8Y transfer event from reserve to designated BSC recovery wallet | Existing reserve tokens, not seizure or minting; final delivery to victim not established |
| Penguin #1509 received at designated Terra recovery address | Successful 17 October transfer and later matching `owner_of` reads | No finding about security of the recipient's signing keys |
| BSC resumed with attacker recipient protection | Four included blacklist/control transactions, expected full-guard reverts before unpause, unpause receipt and pinned later registrations | Payment remains blocked only while the relevant blacklist/guard configuration is retained; no permanent claim cancellation or general safety certification |
| Native proceeds returned from Osmosis | Matched source/return packets, successful receipts, exact unit reconciliation | A successful outbound IBC transfer alone would not establish permanent exit from Terra |
| Original snapshot attacker-address funds included BNB and USDT | Pinned balances and native/accounting closure | Outside funding is separated; no realized fiat receipt or acquisition basis is established |

## Observed system boundaries

- Standard signatures and sender authorization were sufficient for the captured bridge, swap, LP and unwrap paths. The observed economic debits were funded from the signing wallets or their owned positions.
- Cross-chain validation matched deposit data; it did not establish the wallet owner's off-chain consent.
- A token sender restriction affected ordinary CL8Y transfer paths. It was not a freeze of BNB, USDT or Terra native bank coins.
- Bridge cancellation/execution state and source deposit existence are distinct facts. A zeroed destination getter did not mean the source claim was cancelled.
- A token approval changed permission. Successful sales were separate transactions, and their earlier timing mattered.
- Aggregator routes and module-driven activity produced many intermediate events. Net recipient credits, successful call branches and transaction attribution determined economic proceeds.

These are recorded boundaries, not proposed protocol changes.

## Unresolved factual questions

- What signing mechanism produced the unauthorized transactions for each wallet?
- Was the Ledger device involved in the source signature, or was equivalent signing material/path available elsewhere?
- What did the signer-facing transaction prompts, application logs, automation logs and host records show at those times?
- Who controlled the destination addresses, the native funding sources and any relevant off-chain infrastructure?
- What happened beyond the original flow window, the separately evidenced [16 October bridge response](BRIDGE_RECOVERY.md), and the [17–18 October recovery follow-up](RECOVERY_UPDATE_20261018.md), including final delivery from recovery custody? The [CL8Y recovery assessment](CL8Y_RECOVERY.md) addresses the deployed guard path, compensation and long-term migration.
- Did any transient historical permission change occur outside the captured state/event review?

No independent endpoint/device forensic images, host logs, malware analysis, seed-storage audit, or victim interview transcript are in this archive. The original claims about SSH and process history remain source-reported statements. The record contains no finding of fraud, a Ledger defect, or a named attacker.
