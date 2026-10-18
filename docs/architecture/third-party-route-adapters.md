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

[#690 XDex-1–XDex-14](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/690)
is the existing protocol specification. Its latest fee comment changes **300 to
290 basis points**, on top of wrapper and venue fees; its other constraints remain.
This corrects the initial proposed input-basis/ceiling design, which was not
consistent with that specification. No such input fee was implemented.

A route may contain **at most one external pair**, within the existing total hop
bound. A foreign-only route is allowed only through the same guarded executor.
Collect `floor(measured_foreign_ask_delta * 290 / 10000)` in the foreign hop's
**ask asset**, paid to the pinned CMM treasury
`terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2`.
The recipient is neither governance nor the CL8Y pair's treasury. The surcharge
is independent of the trader's CL8Y registry tier. It is not an input-asset debit
or a ceiling-rounded charge. Test dust and exact floor rounding independently.
The whole transaction, including treasury payment, reverts on failure.

Candidate ranking compares final delivered net after the venue's actual AMM fee,
this post-hop skim, required wrapper fees, transfer taxes and hooks. A simulation
that already subtracts the venue fee must not have it subtracted twice. Keep
native network fees separate. Attribute signed components in final-output units
under RC1483. CL8Y-only candidates have no external skim or foreign preference
boost. No multi-path amount splitting is added.

Every external pair requires both independently priced reserves and at least
**$1,000 TVL**, plus at least one canonical LUNC/USTC leg (native or its pinned
wrapper). Every CW20 code ID must remain factory-whitelisted; missing or changed
code, fee or price refuses eligibility. Native uluna/uusd are allowed only in
the specified intermediate role; IBC USDC remains outside this issue. These are
separate external-edge admission rules, not changes to existing CL8Y small-market
reference floors or a new fair-price quality class.

Skip wrap only when native input is actually the first external offer; skip
unwrap only when the last external ask already pays the requested native.
Otherwise include the complete existing wrapper/burn-tax stack, in addition to
2.9% and the venue fee. Never route native user funds through a second taxable
transfer merely to fit an existing CW20 envelope. Native forwarding, native CMM
payment and guarded solo/mixed settlement need an independently verified atomic
protocol. A raw direct external send that skips the skim/delta guards is not a
supported alternative. Native paths remain disabled until those conditions are
implemented and measured.

Terraport and Classic TerraSwap are the initial specified families, using their
verified pinned factories and measured schemas/fees. Garuda, Weso, LuncSwap and
any additional venue remain disabled until factory/pair/swap/simulation identity
and behavior have recorded probes. Terraport/TerraSwap are not Astroport.
External pairs must never be registered as CL8Y listed pairs, enter CL8Y TVL or
volume accounting, or receive CL8Y listed-hop tax semantics. Existing CL8Y hops
retain hybrid execution, blacklist/F6/launch guards and original tax behavior.

## Atomic settlement and trust boundaries

Use an audited on-chain adapter schema with an allowlisted DEX operation enum,
exact token/pair identities, authorized funding, measured ask-delta fee version, ordered legs, recipient,
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
   test exact treasury receipt, rounding/dust, one allowed external hop and rejection of two, tier independence, no extra wallet debit, refund ownership and rollback.
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

The design follows XDex-1–XDex-14 with the later 290-bps correction; it does not
replace the native atomic-execution proof or permit raw/integrator fee bypass.
Source or presentation tests do not establish a deployed adapter. Third-party
integration and 290-bps treasury settlement remain unimplemented acceptance items.
