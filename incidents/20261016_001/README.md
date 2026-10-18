# Case 20261016_001

This is the factual publication archive for unauthorized wallet-signed asset movements involving Terra Classic, BSC, and Osmosis. The requester authorized repository publication and merging to `origin/main`, relaying the victim's permission. Personal names and handles are excluded; pseudonymous wallet addresses and public transaction hashes are included.

The archive identifier `20261016_001` was supplied by the requester. **The original captured incident activity occurred on 2026-10-15 UTC.** The identifier does not move those events to 16 October. The separately dated bridge-response follow-up records completed recovery and control changes on **2026-10-16 UTC**.

**16 October follow-up:** [BNB_FOLLOWUP.md](BNB_FOLLOWUP.md) records the later USDT → BNB sale, KuCoin-attributed external funding, BNB forwarding to a FixedFloat-attributed address, Relay → Ethereum settlement and seven further ETH transfers. Original holdings below remain historical snapshots. No tests/CI are run for this requested documentation-only update.

**CL8Y recovery assessment:** [CL8Y_RECOVERY.md](CL8Y_RECOVERY.md) records the separately requested read-only assessment that the deployed BSC guard has no working forced-transfer path for CL8Y v2, the interim compensation decision from the project's unused CL8Y liquidity reserve, and the long-term recommendation to migrate to CL8Y v3. No tests/CI are run for that requested documentation-only update.

**17–18 October recovery update:** [RECOVERY_UPDATE_20261018.md](RECOVERY_UPDATE_20261018.md) verifies **25,452.503948354102807429 CL8Y** reserve compensation to the designated BSC recovery wallet and **Penguin #1509** receipt at the Terra recovery address. Police reporting remains participant-reported. Private messages are anonymized and paraphrased; the public archive excludes operational recovery details. No tests/CI are run for this requested documentation update.

The public archive covers historical incident transactions and completed recovery; operational recovery details are excluded.

## Record status

| Item | Recorded scope |
| --- | --- |
| First identified unauthorized source deposit | 2026-10-15 02:10:35 UTC, from `[VICTIM_RECOVERY]` |
| Source transactions | 68 selected transactions: 66 affected-wallet transactions, a relay IBC transfer, and its return receipt |
| BSC flow window | 2026-10-15 00:00–09:12:20 UTC |
| Later BSC balance/nonce check | 09:44:22 UTC; balances unchanged, account nonce 56 |
| Exchange email outreach | Requester confirmed sending both prepared FixedFloat/KuCoin emails from `contact@cl8y.com`; no delivery receipts or exchange responses supplied |
| Subsequent response communication | Terra memo transaction included at 10:42:06 UTC on 15 October |
| Completed bridge recovery | 16 October: 10.154338 vFDUSD paid to the newly designated BSC recovery wallet |
| Completed CL8Y compensation | 18 October: 25,452.503948354102807429 CL8Y transferred from project reserve to the BSC recovery wallet; attacker-held tokens not seized |
| Completed NFT recovery | #1509 transfer and recipient ownership verified |
| BSC containment and resumed operation | Both BSC attacker wallets blacklisted for bridge deposits/withdrawals; BSC unpaused at 02:45:37 UTC on 16 October; original nonce 153 still unused |
| Wallet custody statement | `[VICTIM_RECOVERY]`: Ledger, victim-reported; `[VICTIM_A]` / `[VICTIM_B]`: software wallets |
| Root cause | Unauthorized use of signing authority is supported; the mechanism and physical device involvement are unresolved |
| Contract assessment | No exploit or takeover identified in the reviewed flows; this is not a general audit or an all-contract safety certification |
| Publication scope | Factual evidence, statements, methods, accounting, corrections and limitations; the separate CL8Y recovery assessment records read-only observations, an interim compensation decision and a long-term v3 migration recommendation |

## Reading order

- [RECOVERY_UPDATE_20261018.md](RECOVERY_UPDATE_20261018.md) — executed reserve compensation, verified NFT receipt and participant-reported police update, with anonymized provenance.
- [BNB_FOLLOWUP.md](BNB_FOLLOWUP.md) — later BSC cash movements, exchange attribution, Ethereum continuation and recovery contact record.
- [REPORT.md](REPORT.md) — detailed narrative, wallet/contract identifiers and transaction tables.
- [TIMELINE.md](TIMELINE.md) — key events and the correction of the initial investigation cutoff.
- [ACCOUNTING.md](ACCOUNTING.md) — exact unit accounting, dated USD valuation and exclusions.
- [FINDINGS.md](FINDINGS.md) — observations, interpretation boundaries and unresolved factual questions.
- [EVIDENCE.md](EVIDENCE.md) — collection methods, provenance, transformations, confidence and coverage.
- [BRIDGE_RECOVERY.md](BRIDGE_RECOVERY.md) — completed 16 October vault payout, bridge blacklist, unpause, later snapshots and outstanding claim dependency.
- [CL8Y_RECOVERY.md](CL8Y_RECOVERY.md) — read-only guard assessment, why the forced-transfer path fails, interim compensation from the CL8Y reserve and the long-term v3 migration recommendation.
- [COMMUNICATIONS.md](COMMUNICATIONS.md) — the included Terra notice, pending BSC notice and requester-confirmed FixedFloat/KuCoin email outreach, separate from theft accounting.
- [LEDGER_DICTIONARY.md](LEDGER_DICTIONARY.md) — file purposes, units, row counts and attribution semantics.

Selected deployed runtime artifacts and their temporal limits are indexed in [evidence/runtime-artifacts/index.json](evidence/runtime-artifacts/index.json).

Captured responses are under [evidence/](evidence/), derived CSVs under [ledgers/](ledgers/), and redacted source narratives under [sources/](sources/). [evidence-catalog.json](evidence-catalog.json) links published captures to source file digests and describes transformations. [manifest.json](manifest.json) and [SHA256SUMS](SHA256SUMS) record the final published bytes.

## Offline verification

From the repository root:

```bash
python3 incidents/20261016_001/tools/verify_archive.py
```

The standard-library verifier reads local files only. It verifies integrity, relative evidence links, transaction coverage, source debit/fee/net reconciliation, BSC receipt gas, native call-tree payouts, terminal holdings and the dated valuation. It signs nothing, changes no files, and sends no network requests.

RPC/LCD responses are captured observations, not independently authenticated consensus/state proofs. File hashes detect subsequent changes to this archive; they do not establish that every original provider response was accurate. The private originals and the publication derivatives have different digests where identifiers or response proposals were removed.
