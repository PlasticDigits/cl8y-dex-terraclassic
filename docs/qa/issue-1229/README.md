# Launcher origin verification (#1229)

Issue: [#1229](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1229).
Contract: [LO1229-1–LO1229-6](../../../skills/AGENTS_LAUNCHER_ORIGIN.md).
Source results: [verification.md](verification.md).
Existing path: [#606](../../../skills/AGENTS_COMMUNITY_TAX_ENABLE_FEATURE.md).

The token rejects a launcher claim unless the instantiate sender is that
launcher. The launcher records successful token instantiate replies and checks
that membership before invoices, sister creation and binding. Existing catalog
code/admin/launcher-transaction/adopt checks remain in force.

## Source acceptance

- [x] AC1: mismatched instantiate sender rejected (`origin_1229_instantiate_requires_launcher_sender`).
- [x] AC2/AC4: honest create retains origin/CMM admin and manager enable forwards exactly 50 UST1 to CMM (existing launcher crate and inverted #606 tests).
- [x] AC3/AC5: pre-fix self-report fixture with CMM admin fails both TransferTax and AutoV2Lp forwarding; balances/features/binding unchanged (`origin_1229_uncreated_legacy_token_cannot_forward_or_bind`).
- [x] AC6: direct token has null origin; launcher rejects it; manager direct invoice succeeds (`origin_1229_direct_token_retains_manager_invoice_path`).
- [x] AC7 source: three focused catalog attestation regressions pass, including rogue/missing-CMM rejection; production catalog readback remains pending.
- [x] AC8: invariants, #606 deputy guidance and agent skills updated; `make verify-issue-606` passed all 8 steps (crates, audit cases, 7 frontend tests and documentation, including its required retest).
- [x] AC9: token 1.5.0 and launcher 1.1.0 both require columbus-5 migration under their existing CMM/DEX 2-of-3 administrators.
- [x] LocalTerra smoke with deployed new wasm: free create / origin / CMM admin, exact 50 UST1 invoice receipt, mismatched-origin rejection and uncreated-token launcher rejection; [receipt](localterra-smoke.json), 4.19 seconds.
- [x] Scoped pinned optimizer build and source/hash/declared-local checks for both origin wasm artifacts.
- [ ] Production governed migrations and read-only/live honest-path acceptance.

Run `make verify-issue-1229` with a warm owned Cargo target directory. Every
individual test invocation is capped at 59 seconds. It runs the two affected
crates once, including inverted #606 audit tests and origin regressions. Existing
`make verify-issue-606` covers the unchanged frontend invoice utility and smoke
crosslinks. Source test success does not establish deployed wasm.

## Migration and compatibility

1. Build and audit the two wasm artifacts; record exact checksum/code IDs and
   query current token/launcher administrators and launcher pins.
2. Prepare distinct governed token and launcher migration transactions. Token
   same-crate migration uses `{"adopt":null}`; launcher uses `{}`. Update the
   launcher's token-code pin through existing wasm-admin `UpdateConfig` only
   after the new token ID is properly listed. Never list launcher/AutoLP IDs.
3. Launcher migration starts with an empty created-token map. Pre-upgrade and
   adopted tokens remain denied through launcher EnableFeature. Their direct
   manager invoice path remains available; no self-report bootstrap is accepted.
   Do not claim those instances regained the retail launcher path.
4. Verify fresh launcher free-create origin and CMM admin, then an exact invoice
   with CMM receipt. Confirm synthetic mismatched origin remains rejected in an
   owned local environment. Record deployed IDs/checksums and transactions.

This change does not erase legacy stored origin strings during same-crate token
migration; the independent launcher membership gate blocks their use as deputy
authority. Any future historical enrollment needs separately authenticated
creation provenance and review. Coolify deploy cannot complete this on-chain
migration. Keep #1229 open until its runnable and production criteria pass.

## Operator handoff (local encrypted keyring)

The locally prepared token/launcher wasm and matching reviewed checksum
manifest are present in this worktree's ignored `smartcontracts/artifacts/`
directory. The unchanged factory/pair/router inputs were copied only after
matching the existing committed checksums. [Build provenance](wasm-build.json)
and [declared-local compatibility](wasm-local-limit.json) record the actual
outputs. The local preflight needs no network or custody and passes now:

```bash
python3 scripts/ops/launcher-origin-1229.py --check-artifacts
```

Run the following from this checkout after providers recover:

```bash
python3 scripts/ops/launcher-origin-1229.py
python3 scripts/ops/launcher-origin-1229.py --execute
python3 scripts/ops/launcher-origin-1229.py --verify
```

The default command prepares the bounded read-only inventory and immutable
manifest. Execute stores only the required origin wasm, discovers actual code
IDs from receipts, uses the existing keyring transport's concealed passphrase
prompt, pauses affected pairs, migrates managed tokens/launcher, rotates the
template and refreshes F6 pins. It rejects unrelated factory/router/pair/treasury
upgrades. Exact CMM, governance, factory and launcher addresses come from the
existing rollout defaults and are attested against chain state before execution.
No secrets are arguments or echoed. No code ID is fabricated. Receipts stay in
`~/.local/state/cl8y-dex/launcher-origin-1229/`.

Apply that journal's generated `deployment.env` pins to the indexer/frontend
through the existing Coolify deployment workflow. Verify deployment readback
against the new token ID/checksum before restoring recorded original pauses:

```bash
python3 scripts/ops/launcher-origin-1229.py --finalize
python3 scripts/ops/launcher-origin-1229.py --verify
```

This is an operator handoff, not a completed rollout receipt. A missing artifact,
locked keyring, incomplete 2-of-3 custody or failed provider remains an explicit
incomplete check; never mark on-chain acceptance from a prepared plan.

For a fresh source rebuild, use the scoped builder with a new absolute output
directory (`scripts/qa/build-issue-1229-wasm.sh`); it creates only the token and
launcher artifacts in separate compiler invocations and verifies source/hash
provenance. Preserve the exact recorded artifact hashes during deployment.

## Owned LocalTerra smoke

`localterra-origin-1229.py` uses only network `localterra`, new random keys and
independent contracts on the running functional1400 container. It never resets
infrastructure or mutates existing fixture contracts. Prepare/build is separate
from the bounded actual acceptance run:

```bash
python3 scripts/qa/localterra-origin-1229.py prepare
timeout 59s python3 scripts/qa/localterra-origin-1229.py run
```

Preparation refuses to overwrite `/tmp/dex-origin-1229-localterra.json`; preserve
that owned state/receipt when rerunning acceptance. The completed test used token
code **112** and launcher code **111** with exact recorded optimized checksums.
Generated local key material is neither printed nor written to receipts.
The chain's test keyring retains the owned fixture keys. Only a local faucet
transfer funds the new manager; UST1 and launcher instances belong to this run.
