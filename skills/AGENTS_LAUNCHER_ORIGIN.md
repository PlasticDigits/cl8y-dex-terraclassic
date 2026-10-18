# Launcher origin and privileged invoice forwarding (#1229)

Use for community-tax-token instantiate origin, community-token-launcher
EnableFeature, token replies, AutoLP bind or origin migration changes.

Issue: [#1229](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1229).
Sibling: [#606 Enable Feature](AGENTS_COMMUNITY_TAX_ENABLE_FEATURE.md),
[I594 catalog](AGENTS_INDEXER_COMMUNITY_TOKENS.md),
[#626 adoption](AGENTS_FRONTEND_TOKEN_MIGRATE.md).
Verification: `make verify-issue-1229`; [QA checklist](../docs/qa/issue-1229/README.md).

| Invariant | Contract |
| --- | --- |
| **LO1229-1** | `InstantiateMsg.launcher=Some(addr)` is valid only when the validated address equals `info.sender`. Reject before token state writes. `None` keeps direct instantiate available. |
| **LO1229-2** | Launcher `CREATED_TOKENS` membership is written only by a successful authenticated token instantiate reply. Self-reported `GetLauncherOrigin` cannot enroll a token. |
| **LO1229-3** | Before forwarding any UST1 or creating an AutoLP sister, EnableFeature requires created-token membership, manager payer and matching origin. Exact invoice / configured UST1 / MintControl / no-op restrictions stay enforced. |
| **LO1229-4** | AutoLP bind reply independently requires created-token membership. Failed create/enable/bind rolls back membership, invoice and pending state atomically. |
| **LO1229-5** | Keep I594 code ID + CMM admin + anchored launcher transaction or governed adopt provenance. Catalog admission does not grant launcher membership. Direct manager-to-token invoices remain supported by the contract. |
| **LO1229-6** | Token 1.5.0 and launcher 1.1.0 require separate governed columbus-5 migrations. Launcher migration does not infer historical/adopted membership from token queries; those instances remain denied via the launcher. No blanket legacy bypass. Coolify is not a wasm migration. |

New token creation after the launcher upgrade remains the supported launcher
path. Existing tokens retain their direct manager invoice path. Restoring
historical launcher membership requires a separately reviewed provenance-backed
migration, rather than trusting a token's origin string. Do not silently promise
that a migrated launcher can unlock pre-upgrade/adopted instances.

Never whitelist the launcher or AutoLP as pair assets. Do not expose a mainnet
abuse script. Rejection fixtures use only owned synthetic actors.
