# Third-party Terra Classic route adapters

Design for [#1330](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1330)
and its [#690](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/690)
follow-up. This document specifies implementation boundaries; no third-party
adapter or deployed fee settlement is claimed. Current supported routing remains
CL8Y factory routes. [The checklist](../qa/issue-1330/README.md) records incomplete
acceptance. Existing quote rules are [LQ1452](../../skills/AGENTS_LOCAL_SWAP_QUOTES.md),
[RC1483](../../skills/AGENTS_ROUTE_COST_ATTRIBUTION.md),
[RT1381](../../skills/AGENTS_ROUTE_LOCAL_TAX.md) and
[NE1398](../../skills/AGENTS_NET_EXECUTION_FLOORS.md).

## Admission and data ownership

A versioned adapter manifest must pin chain ID, exact factory/pair/router
addresses, wasm code IDs and code checksums, supported execution/query schemas,
asset identity/decimals, tax semantics and an immutable adapter version. Admission
requires independently attested contract information and configuration; symbols,
URL discovery and a third-party factory listing never authorize execution.
CW20 admission uses the existing capability/code allowlist. Unknown transfer tax,
hooks, upgrade state or token semantics refuse execution. A manifest update
invalidates cached candidate eligibility and requires a new review.

Independently scheduled producers ingest third-party reserve/book/config snapshots
through existing bounded provider lanes. A quote request cannot discover contracts,
query a chain or accelerate producer cadence. Every candidate carries its original
observation times and all prerequisite deadlines. Price-only or unsupported quotes
stay indicative and cannot generate a signing payload. Search candidates and total
hops share the existing global bounds; adding a DEX never multiplies them.

## Exact fee and execution economics

The issue requires a flat **290 basis points** (2.9%) independent of trader tier.
The following v1 incidence, input basis and rounding choices are proposed design
decisions, rather than already deployed or issue-specified behavior. They need
contract/economic review with the adapter implementation.

Under this proposed version, an admitted path containing any third-party DEX pays **290 basis points**
(2.9%), independent of the trader's CL8Y registry tier. Charge once per complete
route, rather than once per third-party hop. First-party-only paths do not gain
this surcharge. Calculate in integer raw units with the funding asset's actual
decimals and a published rounding rule: `ceil(gross_input * 290 / 10000)`.
The funded gross input is the trader's authorized ceiling, including this fee;
only the remainder can enter the route. Reject fee-consuming dust and unsupported
fee-token semantics. Any change to that incidence requires a new explicit protocol
version, rather than an off-chain UI adjustment.

The execution adapter must transfer the fee to the protocol treasury atomically
with the swap and authenticate the original payer. It cannot request another
wallet debit after the route begins. Existing native wrap/unwrap principal,
wrapper fees, community tax and post-hook minimum-return rules apply independently.
Third-party tax/transfer behavior must have a measured, supported execution model;
unknown behavior never gets estimated as zero. Route-local tax funding remains
versioned and can spend only the authorized gross input.

Candidate ranking compares final delivered net after all wrapper, DEX, transfer,
surcharge and hook costs in final-output units. Keep the native network fee
separate. Attribute every signed component and reconcile exactly as RC1483
requires. A direct third-party path can lose to a CL8Y middle-hop path after fees;
fixtures must also prove the inverse. Spot-price superiority cannot choose a
winner. No multi-path amount split is added under #1330.

## Atomic settlement and trust boundaries

Use an audited on-chain adapter schema with an allowlisted DEX operation enum,
exact token/pair identities, gross funding, fee version, ordered legs, recipient,
final positive net minimum and original reviewed execution identity. Never permit
arbitrary wasm messages from an API or third-party response. Router/adapter state
must authenticate the caller, enforce allowed operation ordering and repeated-asset
rules, and scope callbacks to the active route. One route owns one non-reentrant
settlement state. Any malformed response, callback replay, balance mismatch,
unsupported hook or insufficient final output reverts the complete transaction,
including the surcharge. Only authenticated original-input residue returned to its
payer can use existing refund exceptions.

Before adding an adapter, review its real CW20/native receive hooks, output
recipient controls, failure behavior, allowances, reserve updates and callback
ordering. Store no unlimited token allowances. A third-party upgrade or pause
revokes positive observations until fresh compatible attestation. Rejection uses
fixed public classes, with no provider/body/actor leakage. No error path may
silently fall back to different legs after review.

## Frontend and agent contract

A local quote must expose exact leg order, input/output units, DEX names from a
static allowlist, surcharge, other fees, order slippage and reference deviation.
The signed message uses the same execution identity and operations as the review.
Retry obtains a new reviewable quote and never signs. Campaign parameters stay
inert in the current URL and do not enter actor metadata, requests or signing.
Errors remain visible and available to screen readers; asynchronous changes use
status/alert regions. Preserve the accessible 30-minute local signing window
without renewing producer observations or other original safety deadlines.

## Delivery and verification

1. Add the manifest/schema and adapter-specific independently scheduled ingestion;
   test changed/unknown code, wrong chain, malformed/late data and cancellation.
2. Add the atomic contract adapter/fee settlement with native/CW20 test fixtures;
   test exact treasury receipt, rounding/dust, single charge over multiple external
   hops, tier independence, no extra wallet debit, refund ownership and rollback.
3. Add off-chain simulation/attribution and exact-net ranking; test both direct and
   CL8Y-middle winners in mixed 6/18 decimals and tax/wrap directions.
4. Bind UI, refresh, progress and signing to the reviewed operation identity. Test
   silent route replacement, query retries, campaign persistence and accessibility.
5. Produce reviewed wasm/checksums, execute governed chain upgrades with receipts,
   update deployment pins, deploy through Coolify, attest served frontend/backend
   revisions and inspect bounded Grafana/Loki and provisioned Prometheus results.
6. Run focused columbus-5 positive/failure quotes, plus explicit funded atomic
   treasury/minimum/rollback acceptance. Keep unavailable checks incomplete and
   retain every executed failure; close #1330 only after its live criteria and
   comment requirements pass.

Source or presentation tests do not establish a deployed adapter. Third-party
integration and 290-bps treasury settlement remain unimplemented acceptance items.
