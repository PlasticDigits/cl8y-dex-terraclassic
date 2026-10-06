# Router trust recovery and settlement lifecycle (#1388)

Issue: [#1388](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1388).
Invariants: [RTTRUST1388-1–RTTRUST1388-10](../../skills/AGENTS_ROUTER_TRUST_LIFECYCLE.md).
[Agent verification checklist](../qa/issue-1388/README.md),
[route funding and rollout](route-local-tax-1381.md).

## Migration and immediate rollout

Router and community-tax-token release **1.3.0** adds settlement identities and
trust recovery. Migrate existing addresses together while affected markets are
contained. The token's stored configuration remains compatible, including adopted
instances; same-crate migration retains balances, rates, roles and origin.
The router counter initializes for pre-counter storage and survives subsequent
migrations. Do not downgrade to code that lacks single-use settlement.

Before any mutation, inventory all factory assets, launcher instances and explicit
community-tax tokens, including unpaired/zero-rate tokens. Each stored
`config.router` must equal the rollout router. Already funded instances must pass
live funding capability checks as well. The immediate rollout rejects a new router
address; it does not automatically rotate tokens. Compare the reviewed local
artifact SHA-256, manifest identity and actual stored-chain checksum, independently
of CW2 version. Refresh pair pins and application registries after migration.

## Deliberate future replacement or revocation

The token's **current chain wasm admin** or **live governance of its configured
factory** can execute `rotate_router`. Manager privileges and settings invoices
confer no rotation permission. Factory governance can recover CMM-owned tokens
directly without impersonating CMM or requiring an arbitrary-execute dispatcher;
CMM's migrate-owned dispatcher alone cannot send this execute message. The same
factory-governance path remains available for immutable token code.

```json
{"rotate_router":{"router":"<reviewed-router>","expected_code_id":12345,"expected_checksum":"<64-hex-character-sha256>"}}
```

The candidate must match both pins, the token's factory, funding version 1 and
settlement version 1. It must be immutable or have its live wasm admin equal
governance queried from the token's configured factory, and have no active route
context. Reviewed code under an attacker admin is rejected: that admin could
migrate it after selection. A CMM token admin need not equal factory governance. Its `last_route_id`
sets the token's replay floor; only future route IDs can settle. Use reviewed
artifact identity, never a checksum merely returned by an unreviewed candidate.
A failed validation leaves binding, exemptions and replay state unchanged.

To revoke routing without assigning a replacement:

```json
{"rotate_router":{"router":null,"expected_code_id":null,"expected_checksum":null}}
```

The obsolete router immediately loses route-transfer authority. Its protocol
exemption is removed unless another valid role or listed-pair membership still
requires it. This does not erase a separately configured manager-directory
exemption. Rates, balances, sinks and other settings stay intact. Contain markets,
verify every token binding and preview, refresh discovery/cache state, and conduct
bounded synthetic funding checks before resuming trading. Partial multi-token
rotation needs forward repair; each token rotation is atomic but separate chain
transactions are not one fleet-wide transaction.

## Settlement protocol and evidence

The canonical router increments a persistent route ID when accepting a route.
Its authenticated context enumerates final delivery and residue refunds with
unique slots. `route_transfer` carries the slot; token execution matches the exact
slot/token/recipient/amount and consumes it before transfer. Each token retains
only its latest route ID and at most five consumed slots. Duplicate slots and old
route IDs fail; identical amounts in separate routes remain legitimate. Whole
transaction failure restores nonce, consumption, balances and taxes.

This bounds replay by an authenticated router; it does not claim arbitrary users
can steal another wallet's balance. Settlement debits the router's owned funds.
Route accounting still excludes existing router dust and intermediate wallets.
Legacy routers lacking settlement identities must be migrated with their tokens.

Trading discovery fetches live code identity before reusing verdicts, and rejects
code changes during discovery. A same-version migration therefore cannot preserve
an old cached capability verdict. Catalog origin/admin/creation/code provenance
remains independently enforced; unchanged-code capabilities and rates retain their
five-second TTL. This change does not resolve launcher spoofing
owned by [#1229](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1229).

Record source checks, synthetic LocalTerra receipts and production receipts
separately. Agent skips are allowed by the checklist; a skip is not a migration
receipt or completed production criterion.
