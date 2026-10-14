# Bounded anonymous live lifecycle and metric receipts

Run only after the exact final frontend/backend deployment is independently
attested by [the existing attester](../../../scripts/qa/attest-trading-quotes-1473.py).
The lifecycle driver checks the attested complete served root/script hashes before
and after, plus public backend health. No prior deployment or repository HEAD is
silently accepted. The driver has no wallet, screenshot or broadcast capability;
non-read methods, blockchain-provider paths and unrelated script/XHR requests are
blocked. Only sanitized response classes, public raw quote output, original
observation/expiry clocks, DOM values/metrics and geometry enter its0600 receipt.
No generation/context digest, actor, credentials, upstream text or raw body enters
its report.

```sh
# The coordinator supplies the independently attested final revision and file.
EXPECTED_SOURCE_SHA="$DEPLOYED_FINAL_SHA" \
DEX_LIVE_ATTESTATION=/tmp/dex-final-attestation.json \
DEX_LIVE_OUTPUT=/tmp/dex1499-live-first.json \
bash scripts/with-node.sh -- node scripts/qa/live-trading-lifecycle-1499.cjs
```

Use `node scripts/qa/live-trading-lifecycle-1499.cjs` through the repository's
Node wrapper if the script is not executable. An optional `DEX_LIVE_CLUNC` may
supply the exact public configured cLUNC execution identity; otherwise the bounded
public token catalog must identify exactly one cLUNC. Ambiguity fails setup.
Explicit native USTC↔CL8Y132, CL8Y↔cLUNC13 and the arbitrary factory UST1↔CL8Y1
controls run unchanged for190seconds each at1280×900 and390×844, at most four
owned pages concurrently. This covers both20s and180s windows in the selected
cohort, with a16minute global wall budget and600 total quote/progress reads.
This does not claim the full amount/protection/manual/wallet/Trade matrix.
Settings/DETAILS and both amount fields are prepared before the original input
clock; measurement never resets that clock. Every200ms sample checks fixed
Pay/Receive/summary/status geometry, one persistent DETAILS, one network-fee row,
no page overflow, exact unexpired display ownership and typed unsupported status.
First useful output is confirmed through two render frames against the750ms
browser budget. The independent initialized API first/retest probes use500ms;
loading, transport errors and typed blockers never count as useful output.

The independent [raw-stage validator](../../../scripts/qa/live-quote-economics-1500.cjs)
checks positive floors, quote-relative market ceiling, original metadata/operations,
selected original reserve/book stages, exact bounded original hop.book_terms
(including makers unused by the actual budget), common-unit commissions, fee-free replay,
signed movement, denominator and attribution reconciliation. Missing advisory
metrics are explicitly unavailable, never zero or signing permission. Its isolated
[attack regressions](../../../scripts/qa/test-live-quote-economics-1500.cjs) require no
production access:

```sh
node --test scripts/qa/test-live-quote-economics-1500.cjs \
  scripts/qa/test-live-trading-lifecycle-1499.cjs
node --check scripts/qa/live-trading-lifecycle-1499.cjs
```

Retain every first/retest failure separately. A valid typed policy negative is not
an unsupported transport outcome, and a monitoring error count cannot establish
quote readiness. No browser request/RPC observation here independently proves
server-side zero-RPC, reference-oracle quality, contract deployment or actual
settlement; those require the separate existing socket/producer/contract/funded
receipts. Unavailable cases stay unchecked. An incomplete or failed run remains
FAIL and cannot close#1499 or#1500.
