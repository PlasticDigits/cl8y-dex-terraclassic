# Source results for #1512

Investigation on 2026-10-18, starting from `0a3c251755a88ee9cbbbbc222fc747811c9da08c`.
[Issue](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1512),
[checklist](README.md), [TP1512](../../../skills/AGENTS_TRADE_PRICE_USD.md).
These are local source results; deployment/live acceptance is separate.

The exact issue fixture fails on original source with `1.21412500000e-14` instead
of unavailable. That regression run has 3 FAIL / 8 PASS: the other two failures
show numeric-prefix acceptance of malformed USD. Both the original omission of
fixture metadata and its corrected exact reproduction are retained in local logs;
only the corrected run establishes the reported double-scaling defect.

The final implementation removes implicit legacy/raw and symbol/oracle fallback,
binds callers to the selected pair, and uses indexed historical factory USD plus
a matching human ratio for inversion. A missing value displays a stable **—**.
Historical candles remain independent. An adjacent empty-chart 24h-close bug now
uses the displayed direction, without becoming the current headline. No backend
API/storage units, market/limit construction, LP accounting or signing gates change.

Source checks (each command/test process capped at 59 seconds):

- [x] Exact original fixture fails before and returns unavailable after; genuine indexed fixture returns approximately `0.005285992968` USD.
- [x] `make verify-issue-1512`: 60 helper/chart tests and 3 actual Trade/Charts page tests pass. Final arithmetic strengthening initially failed because of a missing test import, which was corrected. The two helper suites pass 25 tests; total focused suite is now 61 helper/chart tests plus 3 page tests.
- [x] Matrix: 6/6, 6/18, 18/6, 18/18; both offer/ask and display directions; missing ratio reconstruction and missing USD; malicious/nonfinite/nonpositive/overflow/underflow; round trip; foreign pair, future/invalid-calendar timestamp and invalid decimals/raw amounts.
- [x] Trade phone/desktop and Charts: previous valid → missing USD → orientation switch → indexed recovery → foreign-pair rejection. The chart retains its historical candles and renders unavailable current USD without substituting their close.
- [x] `make verify-issue-524`: all 10 groups pass (172 main tests and 2 selected chart tests). `make verify-issue-680`: all 6 groups pass (121 main tests and 7 selected Trade tests).
- [x] Relevant C543/P522 tape/candle/axis/transaction conversion regression subset: 62 tests pass. Full legacy backend/provisioning verification chains were not run for this frontend-only change.
- [x] Frontend TypeScript passes; scoped ESLint has no errors (two pre-existing LimitOrders hook-dependency warnings).
- [ ] Full issue browser screenshot/reload, network-race/attack and backend API acceptance matrix. Component and existing source regressions do not establish every requested browser/device behavior.
- [ ] Exact deployed frontend/backend revision, build logs and live desktop/phone verification. No deployment is claimed by these source checks.

Actual command output, the retained failing reproduction and source-check logs are
available during this investigation under `/tmp/dex-issues-1206-1512`. The issue
must remain open until its body/comments' complete acceptance is established.
