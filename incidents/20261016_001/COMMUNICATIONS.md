# Observed recovery communication

The original investigation was read-only. A later response transaction is separately evidenced by an included Terra receipt. Its one-uluna transfer and transaction fee are outside the incident theft and gas ledgers.

| Field | Captured value |
| --- | --- |
| Chain | Terra Classic, `columbus-5` |
| Transaction | [1092C2E48B2A1B4A5B8FECE1B42F686B26456227C1A4B4045DADC7142A670183](https://finder.terraclassic.community/columbus-5/tx/1092C2E48B2A1B4A5B8FECE1B42F686B26456227C1A4B4045DADC7142A670183) |
| Included height | 30847615 |
| Included timestamp | 2026-10-15 10:42:06 UTC |
| Result | Code 0 |
| Sender | `terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv` |
| Recipient | `[HACKER_C]`, `terra1gvahgertm8d3ghjmw3ygw958ja99dlyvycm28n` |
| Native amount | 1 uluna = 0.000001 LUNC |
| Declared fee | 11330000 uluna = 11.33 LUNC |
| Gas limit / reported gas used | 400000 / 83923 |
| Memo length | 224 UTF-8 bytes; captured chain limit was 256 |

Exact public memo, quoted as historical communication:

> CL8Y Asset Recovery: Regarding assets associated with the 15 October 2026 incident, please contact t.me/ceramicfingers or contact@cl8y.com to coordinate their secure return. Please preserve remaining assets pending recovery.

The response contacts are intentional public contacts in an included chain memo; they do not identify the victim. The receipt establishes inclusion and content, not that a recipient read it, replied or returned assets.

Two BSC recovery-message commands were prepared/requested during the response, but no corresponding BSC communication receipts were supplied in the captured evidence set. This archive does not assert their delivery or non-delivery. There is no finding of an attacker response or recovery agreement.

Primary capture: [evidence/communications/terra-notice-included.json](evidence/communications/terra-notice-included.json). Executable signing scripts, local keyring metadata, operator inbox credentials/cards and draft broadcast commands are excluded from this publication.

## FixedFloat-address recovery notice — prepared, not broadcast

The requester subsequently authorized a BSC notice to `0x4727250679294802377dD6cA6541B8E459077c95` from the earlier operator sender `0x1E8c3c005EF1374E422cD659C46D427f6B9f5b8F`. Its [exact public text](recovery/fixedfloat-onchain-notice.txt) and [earlier prepared support text](recovery/FIXEDFLOAT_CONTACT.md) are retained. No matching unlocked signer was available to the investigating agent; local signing requires the operator's private-key prompt. No BSC notice transaction hash, signature or inclusion is asserted by this update, and that preparation stage did not include a support submission. The subsequent requester-confirmed emails are recorded below; that confirmation does not establish an on-chain broadcast. A prepared signing command is not evidence of delivery. Any later notice fee remains outside theft accounting.

The separately evidenced **16 October** vault payout, bridge blacklist and BSC unpause are recorded in [BRIDGE_RECOVERY.md](BRIDGE_RECOVERY.md). Those operator response transactions do not establish an attacker reply or agreement to this earlier notice.


## FixedFloat and KuCoin emails — requester-confirmed sending

The requester confirmed both prepared recovery emails were sent from **contact@cl8y.com**. The [prepared email copies](recovery/EXCHANGE_EMAILS_20261016.md) identify FixedFloat (`help@fixedfloat.com`, prepared CC `compliance@fixedfloat.com`) and KuCoin (`support@kucoin.com`). [Statement provenance](evidence/communications/email-sending-confirmation.json) records the evidence class and missing metadata.

This is a requester-confirmed sending record. Exact sending timestamps, original .eml/headers, Message-IDs and delivery receipts were not supplied. No exchange acknowledgement, case number, freeze, disclosure or recovery resulting from these emails is established. The on-chain BSC notice remains prepared without a captured signed/broadcast receipt. The bridge-vault payout in BRIDGE_RECOVERY.md remains separately evidenced and is not attributed to the emails.

## Anonymized participant update — supplied dates 17–18 October 2026

The [redacted statement record](sources/recovery-update-20261018.redacted.json) paraphrases the supplied private conversation under `[VICTIM_REPORTER]` and `[RECOVERY_CUSTODIAN]`. It omits names, handles, health/personal narrative, the identifying attachment filename and verbatim messages. The conversation's timezone was not supplied; its displayed clock times are not treated as UTC evidence.

The victim reported a completed NFT recovery and police reporting. The [chain verification](RECOVERY_UPDATE_20261018.md) confirms #1509 receipt and the CL8Y reserve transfer. Police acknowledgement and attachment contents were not supplied. The public-update text was a draft; publication is not established. Private operational recovery details are excluded. No new message was sent to participants or authorities for this documentation update.
