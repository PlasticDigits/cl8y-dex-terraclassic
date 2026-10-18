# Recovery follow-up — 17–18 October 2026

This update separates public-chain observations from participant reports. The supplied private messages are paraphrased under `[VICTIM_REPORTER]` and `[RECOVERY_CUSTODIAN]`; personal names, handles, medical details and the identifying attachment filename are excluded. Public addresses and transaction identifiers follow the existing case policy. They remain correlatable with public chains.

The messages are dated 17–18 October as supplied, with no stated timezone. Their clock times are not converted to UTC or used to establish chain-event order. All chain times below are UTC. The [anonymized statement record](sources/recovery-update-20261018.redacted.json) preserves the claims and their evidence limits without reproducing the conversation.

## CL8Y reserve compensation completed

The previously recorded [compensation decision](CL8Y_RECOVERY.md#interim-compensation) is now supported by a successful BSC transfer:

| Field | Verified observation |
| --- | --- |
| Transaction | [0xafc19e32c247c95f018f52b6379470a9592fbdf11290af2ec8d78676a62fbf3f](https://bscscan.com/tx/0xafc19e32c247c95f018f52b6379470a9592fbdf11290af2ec8d78676a62fbf3f) |
| Included result | Status `1`, BSC chain ID `56`, block **128323680**, **2026-10-18 07:38:36 UTC** |
| Token | Existing CL8Y v2, `0x8f452a1fdd388a45e1080992eff051b4dd9048d2` |
| Source | Project genesis/liquidity reserve `0xfac4c56258941d445afda6bb2fa87b493a65b8a1` |
| Recipient | Designated BSC recovery wallet `0x21ddb4bcb5f00430cb39d5fea2d382bbeaa076ba` |
| Transfer event | **25452503948354102807429 base units**, or **25,452.503948354102807429 CL8Y** at 18 decimals |
| Operator gas | `120087 × 60000000 = 7205220000000` wei, or **0.00000720522 BNB**, outside attacker/theft gas accounting |

The transaction input and CL8Y `Transfer` event agree on the recipient and exact amount. The [primary transaction, receipt and block capture](evidence/recovery-20261018/cl8y-compensation-provider-1.json) supports inclusion and timing. A [second provider capture](evidence/recovery-20261018/cl8y-compensation-provider-2.json) agrees on the transaction but denied the receipt request with HTTP 403; it is not a second successful receipt verification.

This was a transfer of existing project reserve tokens, matching the frozen attacker balance. It was **not minting, seizure of the attacker's tokens, or a v3 migration**. At the later [BSC custody snapshot](evidence/recovery-20261018/bsc-custody-state.json), block **128324235**, **2026-10-18 07:42:46 UTC**:

| Holder / control | Observed state |
| --- | --- |
| Recovery wallet CL8Y | **25,452.503948354102807429** |
| Recovery wallet vFDUSD | **10.154338**, `1015433800` base units at eight BSC decimals |
| Project reserve CL8Y | **900.981440405349584639** |
| `[HACKER_A]` CL8Y | **25,452.503948354102807429**, still in the attacker address |
| Sender-blacklist entry for `[HACKER_A]` | `true` in the previously identified CL8Y `BlacklistSenderOnly` module |

`[RECOVERY_CUSTODIAN]` reported that the compensation and previously recovered vFDUSD would be held in the recovery address pending a final recipient address. The snapshot supports those holdings at that block; it does not establish secure key custody, a later balance, or final delivery to the victim. The vFDUSD is the [16 October net-principal recovery](BRIDGE_RECOVERY.md), not a second recovery on 18 October. Historical theft amounts and the dated USD valuation remain unchanged.

The reserve's earlier **332,069 CL8Y** assessment balance belongs to block 128108592. The later reserve balance is not reconciled by the compensation transfer alone; other reserve activity was not reconstructed in this bounded follow-up and is not attributed to the incident.

## Completed NFT recovery

Penguin #1509 was transferred from the previously identified source address to the designated Terra recovery address `terra1gjd5r45mv334d0qm5mkqhhj64q8fafw3kg62du` at **2026-10-17 00:27:56 UTC**, height **30870523**, result code `0`.

The transaction is [8BAF1EBFEADE35117F21DD53610718A3BD8FB8B4B3D88E558D3F1BAE543C21C6](https://finder.terraclassic.community/columbus-5/tx/8BAF1EBFEADE35117F21DD53610718A3BD8FB8B4B3D88E558D3F1BAE543C21C6). The [selected direct transaction response](evidence/recovery-20261018/penguin-1509-transfer.json) preserves the successful message and matching transfer event. NFT contract: `terra1sc8tz0delrw32f58nhaxqj4k2gar9thnhlwpzx`.

Later ownership reads name the designated recipient: [Hexxagon](evidence/recovery-20261018/penguin-1509-provider-2.json) returned the requested Terra height **30889465**; [PublicNode](evidence/recovery-20261018/penguin-1509-provider-1.json) agreed on the owner but omitted response-height metadata. The [block header](evidence/recovery-20261018/terra-latest-block.json) identifies `columbus-5` and that height. These records establish completed recovery, not security of the recipient's signing keys.

## Participant-reported police update

The supplied messages include a draft public update stating that the incident had been reported to police, alongside discussion of police reporting that day. No police reference, acknowledgement, report copy or independently verifiable public-post URL was supplied. Police reporting is recorded as **participant-reported**, without inferring a completed investigation, attribution or asset recovery. A recovery-update attachment was mentioned, but its contents were not supplied; no technical or forensic conclusions are attributed to it. Personal and medical narrative is omitted.

These statements do not establish the key-theft mechanism, malware findings, involvement of a hardware device, or a new contract exploit. The original [findings and uncertainty](FINDINGS.md) continue to apply.

## Publication scope

The public archive covers historical incident transactions and completed recovery. Operational recovery details are excluded. Provider responses are captured observations, not independently authenticated consensus proofs. The evidence catalog records extractions; publication manifests cover the resulting bytes. **No software tests, archive verifier, CI, build or transaction simulation was run for this update, as requested.**
