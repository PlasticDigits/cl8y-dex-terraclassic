# Chain identity for indexer reads and treasury ticks (#1392)

Implementation: [PR #1413](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1413).

Issue [#1392](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1392).
[NET1392 invariants](../../skills/AGENTS_CHAIN_ATTESTATION.md),
[agent checks](../qa/issue-1392/README.md) and
[verification receipts](../qa/issue-1392/verification.md) define acceptance.

Implementation: [LCD identity](../../indexer/src/lcd/chain_identity.rs),
[LCD data boundary](../../indexer/src/lcd/mod.rs),
[treasury preflight](../../scripts/lib/chain-attestation.py),
[shared shell policy](../../scripts/lib/chain-attestation.sh),
[transaction helpers](../../scripts/lib/terrad-host.sh).

## Indexer configuration

Set `EXPECTED_CHAIN_ID=columbus-5` alongside production `LCD_URLS`.
Set `EXPECTED_CHAIN_ID=localterra` with explicit local LCD URLs for LocalTerra;
`scripts/deploy-dex-local.sh` writes both. Missing/empty expectation is a startup
configuration error, including dev mode. Do not copy mainnet contract/code pins
into a local chain just because numeric code IDs or addresses coincide.

Startup examines every LCD. Each data read checks the selected endpoint again;
a previously successful handshake does not grant indefinite trust. Returned
block headers must also contain the expected `chain_id`; an expected node-info
response cannot excuse a contradictory block. Historical block timestamps remain
readable; treasury applies its separate current-header freshness policy. Unavailable,
wrong-chain and malformed identity responses fail closed and use the bounded
cooldown/failover path. Recovery must pass attestation again. No endpoint means
no usable chain data. The cheap `/health` endpoint reports expected/observed identities, verification
status and `checked_at` from the last actual observation, without provider I/O; cached values
must retain their network namespace and cannot authorize a financial action.

Set the expected chain before restarting existing indexer deployments. This PR
requires an indexer rebuild/restart and updated environment; it has no schema or
CosmWasm migration. Existing analytics stored before this change are not
retroactively authenticated. Separate network deployments must use separate
Postgres databases: an in-process cache namespace does not isolate database rows.

## Treasury configuration

The shared host helper uses the explicitly configured `TERRAD_HOST_CHAIN_ID`
(default `columbus-5`) as its expected network; `TERRAD_HOST_EXPECTED_CHAIN_ID`
may declare the same expectation explicitly. The script's selected LCD and
`TERRAD_HOST_NODE` must both serve that network. Fallback RPCs use exactly the
same policy. The selected script LCD remains authoritative for its REST reads.

| Setting | Policy |
|---|---|
| `TERRAD_HOST_LCD_URL` | LCD selected for generic host helpers; named treasury scripts set their own LCD |
| `TERRAD_HOST_RPC_FALLBACKS` | Optional comma-separated alternate RPCs; every selected candidate is checked |
| `TERRAD_HOST_MAX_BLOCK_AGE_SEC` | Maximum header age, default 120 seconds |
| `TERRAD_HOST_MAX_HEIGHT_LAG` | Maximum LCD/RPC latest-height gap, default 5 blocks |
| `TERRAD_HOST_ATTEST_TIMEOUT_SEC` | Bounded per-request timeout, default 10 seconds |
| `TERRAD_HOST_IDENTITY_CONTRACTS` | Protocol addresses whose code IDs and hashes must agree through LCD/RPC |
| `TERRAD_HOST_IDENTITY_PINS` | Optional reviewed JSON file path, mapping address→`{code_id,checksum}` pins; never infer a reviewed pin from the queried endpoint |

Use explicit local chain, LCD/RPC and deployed contract settings for LocalTerra.
The defaults retain Columbus-5; endpoint overrides never change the expected
chain implicitly. Both dry runs and live ticks perform trust preflight before
financial reads/preparation. Every host transaction command repeats the check,
including unsigned generation, signer commands, multisign, broadcast and retry.
If preflight fails, correct the endpoint/configuration and rerun; do not bypass
it with dry-run or accept cached identity after a timeout.

Code identities are independently corroborated over LCD and RPC and retained as
an immutable per-run baseline. Configured reviewed pins add protection against
an initially incorrect agreeing view. A planned legitimate code migration needs
updated reviewed pins and a fresh process; do not silently reset a live baseline.
The fee-treasury rotation script intentionally migrates factory/pair code, so
its shared guard verifies network/header identity without freezing those planned
migration targets in an automatic baseline; configured reviewed pins still apply.
Generic host commands corroborate the contracts explicitly supplied by their
caller. Named mint/buyback/LP scripts supply their protocol contract sets.
Dynamic route pairs are added to each tick baseline; core protocol identities
remain inherited from startup.

Signing remains in the existing keyring/multisig workflow. The final preflight rechecks anchor age after protocol-code reads, so a slow
response cannot outlive its freshness limit. Network verification
does not increase the spend envelope or waive route, price or min-return checks.

## Remaining trust and evidence

TLS protects transport to the configured provider; chain ID is a string reported
by that provider. Shared-height block and wasm-code agreement catches mismatched
views and one-sided spoofing, but two colluding or commonly backed providers can
agree on fabricated same-chain data. These checks are not a cryptographic
light client and do not fully authenticate a price/config response after the
header check. Continue address/hash pins and spend/quote limits; use genuinely
independent providers and reviewed immutable pins where possible.

Read-only acceptance compares actual network responses without preparing or
broadcasting transactions. Preserve revision, UTC time, exact commands, endpoint
indices, expected/observed networks and freshness scope. Successful receipts expose a zero-based `rpc_candidate_index`; the shell
resolves that candidate from its private configured list. Provider URLs are
excluded from receipts and operational headers because paths can carry API keys.
Publish only public or synthetic endpoint/contract identities and never credentials. Deployment and
signing evidence are separate from local verification.
