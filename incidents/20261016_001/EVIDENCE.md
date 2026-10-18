# Evidence provenance and collection methods

## Evidence classes

| Class | Location | What it establishes |
| --- | --- | --- |
| Captured public-chain transactions and receipts | `evidence/terra-transactions/`, `bsc-transactions/`, `bsc-releases/`, `bsc-native-funding/`, `osmosis/` | Recorded transaction content, included result, transfer events and reported block/time |
| Captured execution/state queries | `evidence/bsc-call-traces/`, `terra-state/`, `bsc-state/` | Provider-returned call trees, configuration, permissions, balances and historical/current observations |
| Corrected calculations and independent checks | `evidence/accounting/` | Reconciliations, bounded query coverage, nonce completeness, native balance closures and interpretation boundaries |
| Derived edge/transaction tables | `ledgers/` | Structured views of the captured evidence, with raw units and explicit scope |
| Source-supplied narratives and export | `sources/` | What was originally reported and the later export that expanded the investigation; these are not independently verified host-forensic findings |
| Completed bridge response | `evidence/bridge-recovery-20261016/`, `ledgers/bridge-response-transactions.csv` | Six included BSC response transactions, payout event, blacklist deployment/registration, pre-unpause verification and later state; separate from theft accounting |
| 17–18 October recovery follow-up | `evidence/recovery-20261018/`, `sources/recovery-update-20261018.redacted.json` | Reserve compensation receipt and later custody, NFT transfer/ownership reads, and separately classified anonymized participant reports |
| Later public notice | `evidence/communications/` | Included response transaction and memo, outside theft accounting |
| Dated spot quotes/calculation | `evidence/valuation/` | USD reference prices and arithmetic at the recorded quote times |

The source narratives were supplied as a Markdown report and a PDF. Both contained victim identifiers and response proposals. Their published derivatives replace personal identifiers with `[VICTIM_REPORTER]` and omit response requests/proposed next steps. The binary PDF and original named Markdown are not in Git. Original-file byte sizes and SHA-256 digests, derivative digests and transformation descriptions are in [sources/original-document-digests.json](sources/original-document-digests.json).

The CL8Y export's nametag fields and unnamed trailing column were removed. Its transfer fields, hashes, amounts and dates remain unchanged. Its USD marks are source-export annotations, not realized sale proceeds. The original and derivative digests are recorded separately.

The victim's reported wallet types and authorization clarification are recorded in [sources/victim-statements.json](sources/victim-statements.json). The requester later relayed permission for repository publication. No identifying interview transcript or medical/personal chat narrative is published.

## Collection scope and methods

- **Terra Classic:** LCD transaction-by-hash and indexed transaction queries collected selected historical transactions for the two software wallets and the Ledger-labelled source. Captured current/historical smart queries, contract information/code history, token minters/allowances, pool settings supplement transaction traces. The source transaction corpus includes the IBC send and return transaction.
- **BSC:** Historical `eth_getLogs` searches filtered both directions for the two reported recipients, without restricting token contracts. The expanded earlier interval also captured approval events. `eth_getTransactionByHash`, receipts and block headers resolved transactions and timing. All attacker-originated account nonces 0–55 were retrieved, including failures.
- **Native BNB:** Full `debug_traceTransaction` call trees for all 20 successful sales supplied actual credits. Branches with an error, descendants of reverted branches, and non-transferring DELEGATECALL/STATICCALL apparent values were excluded. Three incoming native payments were localized by historical balances and verified against direct transaction/receipt data. All 55 unique own-transaction-block balance checks close without unexplained residuals.
- **Bridge:** Source fee/burn events and canonical cross-chain identifiers were matched to destination release receipts and withdrawal getters. The earlier nonce150 source was discovered from its actual BSC release rather than inferred from timing. Nonce151 was not linked to these victim/destination addresses and is outside the incident tally.
- **Osmosis:** Three provider responses captured the same receive/swap/return transaction. Source/return packet sequences, channel directions, receivers, denomination traces and acknowledgements matched. Incident-specific transfer edges were separated from the protorev module's later backrun in the same transaction.
- **Authorities:** Current Terra admins, application governance, token minters, multisig constituent public keys, current BSC authorities and selected historical control comparisons were inspected. This does not constitute an exhaustive transient-role history or an independent audit of all executable contract code.

## Provider and timing limits

Initial public-node and Binance RPC services denied some historical methods or ranges. Alternative archive-capable endpoints supplied the missing historical receipts, logs, balances and call trees. Rate-limit and provider failures occurred during collection; corrected successful captures, rather than failed calls or empty assumptions, support the accounting.

The capture set used PublicNode, Hexxagon, NodeReal, bloXroute, Blockmachine, and Osmosis LCD services including the Osmosis, Polkachu and PublicNode endpoints at different stages. Credential-bearing provider URL components are omitted; no provider token is required by the offline verifier. Transport headers other than dates/content type/query height were removed where present. Provider availability today is not asserted.

Historical Terra LCD reads were repeated with distinct query URLs after a height-header/cache ambiguity was observed. A separate ABCI historical-state path was pruned. The validated LCD snapshots and the explicit limitations are retained; the pruned path does not count as a successful state observation.

The initial 04:00 UTC BSC cutoff missed the earlier CL8Y wave. That investigation error was corrected after the additional export. The initial no-outgoing-token conclusion is not retained as an incident-wide finding. Corrected results include the 02:10 source deposit and 20 earlier sales.

Embedded `collected_at_utc`, `reviewed_at_utc`, provider dates, block timestamps and transaction timestamps have different meanings. Where a capture lacks an explicit collection time, its exact collection instant is not established by the filename or this archive's creation time. Snapshot rows are not evidence of continuing controls or balances.

## Integrity and transformations

[evidence-catalog.json](evidence-catalog.json) records the private source basename/digest for each selected published capture and describes extraction or redaction. It does not expose local source paths. Full unrelated block bodies, duplicate outside-window account histories, superseded provisional calculations, failed HTML probes, credential-bearing helper scripts, local keyring material and executable broadcast tools are excluded.

Captured source digests and published-file digests can differ. This is intentional where a capture was extracted from a larger query result or where transport metadata, identifiers or proposed response actions were removed. Captured CSV line endings are preserved by the case-local `.gitattributes`; this prevents Git newline conversion from changing the ledger byte digests across clones. The final published bytes are covered by [manifest.json](manifest.json) and [SHA256SUMS](SHA256SUMS). These are publication integrity records, not a claim of a pre-existing forensic acquisition seal.

Raw on-chain metadata, memo strings and token names are untrusted evidence. Their inclusion is not an endorsement of referenced sites, token values or sender identity. Source-public wallet addresses, keys and transaction signatures remain correlatable with public chains; omitted personal names do not provide anonymity.

## Reproduction boundaries

The offline verifier uses Python's standard library and performs no network requests or signing. It verifies SHA-256 publication integrity, data/receipt relationships, integer/decimal accounting and the supported call-tree interpretation. It does not independently prove consensus inclusion, re-execute a blockchain VM, validate all signatures against historical consensus state, or recompute EVM Keccak cross-chain hashes. Cross-chain event identifiers are matched between preserved source/destination evidence; the earlier investigation's separate hash-reconciliation result is retained.

The archive additionally contains ten gzip-compressed Terra WASM artifacts, fetched by immutable code ID and verified against the returned SHA-256 code checksum, plus sixteen BSC runtime bytecode captures. Their capture times, queried block/hash for BSC, content digests and source methods are recorded in [evidence/runtime-artifacts/index.json](evidence/runtime-artifacts/index.json). These were retrieved during archive preparation after the incident: immutable Terra code content can be tied to captured historical code IDs, while BSC bytes describe the later pinned archive-time block at the previously recorded addresses. They are not newly established incident-time EVM state proofs or a complete source/dependency reconstruction for every deployed contract. No host/device disk image, memory capture, signing-device diagnostic log, malware sample or independently authenticated original interview is available here. These are limitations of the record, not proposed fixes.

## Publication secret-scan classification

The default credential scanner initially reported 226 matches in public-chain evidence. Independent review classified 33 distinct values:20 EVM asset addresses,6 compressed public keys,6 IBC event attribute names and1 IBC client ID. Their exact occurrences and classifications are recorded in [evidence/scanner-public-values.json](evidence/scanner-public-values.json). The repository scanner configuration contains anchored exact-value exceptions for these known public values; it contains no evidence-directory exclusion. The subsequent 8.18.4 scan reported no leaks. The hosted 8.30.0 scanner additionally detected 47 occurrences of 7 other compressed secp256k1 public keys containing plus or slash characters. Independent review verified every additional value in a Cosmos public-key field, its 33-byte compressed encoding and curve membership. Those 7 exact values are also excepted, bringing the classification inventory to 40 distinct public values. The exact-version scan and dummy-secret rejection fixture were run for the initial publication. They were not rerun for the subsequent documentation-only update. This records scanner classification, not a claim that pseudonymous chain data is anonymous.

## Follow-up acquisition on 16 October 2026

[BNB_FOLLOWUP.md](BNB_FOLLOWUP.md) distinguishes the original cutoff from later receipts. Selected BSC transactions cover all six additional source nonces 56–61, the later external funding payment and the forwarder's nonce 0. The swap's full call tree supplies actual native credits. Pinned source balances reconcile the new BNB equation. The Relay response was filtered by the exact BSC deposit hash; its destination hash was independently retrieved from Ethereum RPC with a successful receipt and matching order ID. The next Ethereum transfer and seven subsequent transfers were separately captured. The forwarder's tiny residual is explicitly unresolved rather than assigned a fabricated sender.

Public contact/label source URLs and capture times are retained. No private provider URL, signer/keyring material, operator inbox identifiers, unrelated service request history or executable response script is published. No tests, archive verifier or secret scanner were executed for this update, in accordance with the explicit requester instruction; publication manifests are freshly computed hashes, not a claim of test success. The existing verifier's financial assertions continue to describe the original episode and valuation.

## 16 October bridge-response update

[BRIDGE_RECOVERY.md](BRIDGE_RECOVERY.md) adds newly captured public BSC transaction/receipt/header records for the vault payout, four blacklist operations and unpause. Capture and block timestamps are distinct. The new records retain public transaction/receipt fields and omit unrelated block-body transactions; they contain no signing key or executable deployment script. The pre-unpause verification and pre-recovery assessment captures preserve read-only observations from the response session; the post-unpause state uses explicit BSC block 127901098.

The requester designated two new recovery addresses; only the BSC token payout is evidenced as completed. Original source ledgers and their row counts remain unchanged. The response ledger is separate, and subsequent snapshots do not retroactively extend the original complete BSC flow window or its valuation. The older offline verifier was not extended to assess these new response semantics. **No tests, archive verifier, CI pipeline, contract build or new transaction simulation was run for this documentation update, as requested.** Public receipt/state reads and documentation/integrity-file preparation are separate from those response-session checks.

## Requester-confirmed exchange email outreach

[evidence/communications/email-sending-confirmation.json](evidence/communications/email-sending-confirmation.json) records the requester's statement that both prepared emails were sent from `contact@cl8y.com`. [The retained prepared text](recovery/EXCHANGE_EMAILS_20261016.md) is not an original sent-email export. Exact sending times, headers, Message-IDs, delivery receipts and exchange responses were not supplied. Sending is reported by the requester; delivery, acknowledgement, preservation/freeze actions and email-caused recovery are not independently established. No software tests, archive verifier or CI were run for this combined documentation publication, as explicitly requested.

## 17–18 October recovery update

[RECOVERY_UPDATE_20261018.md](RECOVERY_UPDATE_20261018.md) adds read-only BSC compensation transaction/receipt/header and pinned custody captures, a selected Terra #1509 transaction response and recipient ownership reads. The primary BSC provider returned a successful compensation receipt; the second provider returned matching transaction data but denied the receipt request. Hexxagon returned the requested Terra height 30889465 with #1509 recipient ownership; PublicNode agreed on the owner without response-height metadata. No full-chain NFT history census is claimed.

The statement derivative excludes names, handles, medical/personal narrative, exact chat clock times and the identifying attachment filename. It preserves relevant operational claims under pseudonymous roles; public-chain identifiers remain correlatable. The original conversation and attachment bytes are not published, and the attachment was not available for review. Police reporting has no independent supporting records here. The draft public statement is not asserted to have been published.

New captures retain public endpoints, explicit requests, collection times, selected transport date/content-type/height metadata and returned observations/errors. Unrelated block bodies, duplicate logs and transaction signing fields were omitted from selected Terra derivatives; transformation notes and canonical-JSON source digests identify those extractions. The catalog and publication hashes were updated. **No tests, archive verifier, secret scanner, CI, build or transaction simulation were run for this documentation-only update, as requested.**
