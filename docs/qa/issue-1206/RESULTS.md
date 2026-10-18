# Local source results

2026-10-18, based on `0a3c251755a88ee9cbbbbc222fc747811c9da08c`.
[Issue](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1206),
[checklist](README.md), [AD1206](../../../skills/AGENTS_ACTIVITY_DIGEST.md).

- [x] Real HTTP/isolated Postgres fixture: all surfaces, independent parent/fill counts, UTC bounds, current ten-source enum and original seven prefix, OpenAPI, empty/today and anonymity.
- [x] Query/SQL-injection/filter/CSV/duplicate abuse and method rejection; closed database returns sanitized 500.
- [x] Mixed priced/unpriced source and token nulls preserve priced headline. Ten assets return eight rows plus other; a 10,000-swap day stays below 8KB with no event array.
- [x] Three library checks: strict day grammar, canonical idle/unpriced output, no SQL actor/tape dependencies.
- [x] Independent agent review found no blocking privacy/grain/query defect.
- [ ] Deployed exact-revision digest/OpenAPI, production timing and live completeness. Local results do not establish those criteria.

The first three-test integration run passed in 2.04s. Adding stronger fixtures
exposed a test-only overlong symbol (the assets schema caps symbols at 20 chars)
and a lowercase SQL guard compared with uppercase text. Both fixtures/guards were
fixed. A later run exhausted its 59s limit during shared-server migration setup
amid severe disk contention, before product assertions. No failure was relabeled
PASS. The final four-test run uses a separately owned disposable loopback Postgres
with bounded memory/CPU and temporary data, and passes in **1.64s**; the three
library checks pass. Every actual test command has a 59s ceiling. Compilation
is separate. The ordinary per-process child-DB harness resets only owned state.

The issue predates three canonical fee paths added by #1209/#1210. This change
explicitly reconciles that stale exact-seven expectation with current
`FeeSource::ALL` (ten), retaining the original seven in order. It does not alter
trailing fees, the Llama API or the public-chain-linked event tape.

The closure audit added the required digest-specific socket-peer governor check.
[All five digest cases passed](digest-governor-local.json) in4.98s on a new owned
loopback PostgreSQL fixture, including actual HTTP200→429 with integer
`Retry-After`. Only that container/volume was removed; no production request ran.
The required hosted security matrix remains a separate CI receipt.
