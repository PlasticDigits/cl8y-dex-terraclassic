# Source verification for #1229

Date: 2026-10-18T04:11:36.638187+00:00. Base checkout:
`0a3c251755a88ee9cbbbbc222fc747811c9da08c` plus the origin hardening change.

The issue body and all comments were read using openbao-tools `bao-exec` and
`fj`. No existing implementation or replacement issue was found.

| Check | Result |
| --- | --- |
| `CARGO_TARGET_DIR=/home/answorld/repos/cl8y-dex-terraclassic/smartcontracts/target timeout 59s make verify-issue-1229` | PASS: 128 token, 11 launcher and 12 audit tests, including 3 origin regressions |
| Same owned warm target, `timeout 59s make verify-issue-606` | PASS: 8 verification steps, including its required retests, 2 inverted launcher cases and 7 frontend invoice tests |
| Scoped `git diff --check` | PASS |
| Python operator wrapper behavioral tests + compilation and verifier `bash -n` | PASS: 5 tests cover plan/execute/verify/finalize modes, real artifact identity/hash checks, tampering, unrelated migrations and invalid modes and local preflight without network/custody |
| Focused catalog attestation tests | PASS: 3 source attestation cases; production catalog readback remains pending governed migration |
| LocalTerra deployed wasm smoke | PASS in 4.19 seconds under timeout 59s; token112/launcher111 match optimized checksums; free create/origin/CMM admin, exact50UST1, mismatched origin and uncreated launcher forwarding covered |
| Scoped pinned optimizer build, source provenance, SHA-256 and declared-local compatibility | PASS: token max locals 59; launcher max locals 64; 100-local cap |
| `python3 scripts/ops/launcher-origin-1229.py --check-artifacts` | PASS against the actual token/launcher and unchanged core wasm inputs; no network/custody |
| Production token/launcher migrations | Not executed; operator workflow provided |

The first source test run exposed old test fixtures that set `launcher` while
instantiating from the manager. Fixtures now instantiate from their declared
launcher. After that correction, all affected crate tests passed. No database or
chain infrastructure changed during these in-memory regression runs.

No production deployment or transaction was attempted in this subtask. Operator
handoff: [README](README.md#operator-handoff-local-encrypted-keyring). Do not close
#1229 based on source checks while the deployment and full issue acceptance
remain incomplete.

Token 1.5.0 artifact SHA-256:
`81161a25d87a76a7527d2bd37dd4ec50484cee1a2506bf5a35166775cd0fbcc2`.
Launcher 1.1.0 artifact SHA-256:
`743e09cf42674b116247a0e0208619088d6f1e30c1a95e273448ab6687b1b25c`.
Both used the pinned workspace-optimizer 0.16.1 image, fresh owned build/registry
volumes, separate crate invocations and `wasm-opt -Os --signext-lowering`.
[Build provenance](wasm-build.json) records exact source, toolchain and output
hashes. [Declared-local check](wasm-local-limit.json) is PASS. These establish
compilation/bytecode compatibility checks only, not live VM simulation or chain
migration.

[Owned LocalTerra receipt](localterra-smoke.json): blocks 2278830–2278850.
Provisioning initially exposed harness gas-floor and first wasm inclusion timing
assumptions; those were corrected before acceptance. The first acceptance attempt
rejected the fixture's spaced token name under the existing ASCII identity
policy. Renaming the synthetic fixture to `OriginSmoke` corrected the harness;
the final affected acceptance run passed without contract changes. New owned
contracts isolate the run; no shared infrastructure reset was performed.

The commit hook's Clippy `op_ref` finding was corrected by comparing the borrowed
launcher directly with `info.sender`. All151 affected contract/audit tests and
scoped Clippy (the hook's warning flags) then passed. A fresh pinned two-contract
build verified the final source inputs; token bytes changed, so the LocalTerra
smoke passed again against the final checksum. The original build and smoke receipts
remain preserved under their original `/tmp/dex-origin-1229-*` paths.
