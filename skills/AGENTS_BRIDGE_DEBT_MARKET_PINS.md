# Bridge-Debt market identity and issuance (#1393)

Use this protocol when changing `bridge-debt`, `bridge-debt-token`, their migration
messages, or the USDT/UST1 deployment script. [Issue #1393](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1393),
[runbook](../docs/runbooks/deploy-usdt-ust1-lp.md),
[agent checklist](../docs/qa/issue-1393/README.md),
[security audit](../docs/contracts-security-audit.md#bridge-debt-market-pins-1393).

| ID | Invariant |
|---|---|
| BD1393-1 | Each market pins its own receipt/underlying code IDs. `SetDebtCodeId` affects future registrations; approving another underlying code cannot refresh existing pins. |
| BD1393-2 | Registration requires an approved underlying implementation and an authentic receipt code. Debt receipt supply and debtor balance equal outstanding; debtor, issuer minter and decimals match the market on Issue, Repay and Recover. |
| BD1393-3 | Single-minter approved code uses exact `Minter` address/cap. Multi-minter approved code additionally exposes a complete bounded `Minters` inventory; pin the sorted set (maximum 30), primary and cap. At a full 30-entry page, query past its final address to prove exhaustion: the approved template clamps page size to 30. Issuer must be authorized. Drift fails closed, including removal of issuer or addition of an unexpected minter. |
| BD1393-4 | Zero/minimum controls never imply unlimited issuance. Registration has no limits and Issue is disabled until configured. Positive raw-unit per-issue, outstanding and epoch caps are mandatory. Pause or pending upgrade disables Issue. |
| BD1393-5 | Epoch = block timestamp / configured duration. Duration cannot change after first configuration. Reconfiguration, repayment and pause toggles never refund consumed epoch quota; a later fixed epoch resets only its own counter. |
| BD1393-6 | Only the issuer mints/burns receipts, always to/from the immutable debtor. Transfer, Send, holder Burn, allowances and receipt UpdateMinter are rejected. |
| BD1393-7 | Owner proposes exact new code IDs; acceptance requires those live implementations and unchanged debtor, minter inventory/cap, units, accounting and underlying supply. Issue remains disabled during proposal. Supported Repay adjusts the pending supply snapshot by its burn; unrelated mint/burn requires cancellation/reproposal. |
| BD1393-8 | Migrations verify cw2 contract identity and explicitly supported semantic versions; reject newer/wrong contracts. Every legacy market needs reviewed `legacy_pins` containing full `MarketPins`; never automatically trust current live code or partially write migrated state. |
| BD1393-9 | `UpdateOwner` proposes only; the recipient calls `AcceptOwner`. Current owner may cancel. Proposer remains owner until acceptance. |
| BD1393-10 | Issue/Repay are atomic same-raw-unit paired mint/burn operations. `Recover` returns accidental plain transfers without changing outstanding. Pause/caps constrain Issue; valid Repay and Recover remain available under consistency checks. |

Code approval is a trust decision about the actual wasm implementation, including
its answers to CW20 queries. A matching code ID is not a proof of honest economic
behavior. Governance can approve future code, adjust caps/pause, propose upgrades
and recover underlying held by the issuer. Wasm admins can migrate contracts,
including replacing the issuer; these remain residual governance powers.
Other legitimate underlying minters (for example the bridge) can mint/burn outside
this debt ledger. Additional Bridge-Debt controls do not control remote custody.

Columbus-5 1.1.0 migration, repayment, and recovery stay with
[REL1421-6](AGENTS_RELEASE_OPS_1421.md) and
[#1421](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1421). A planned
journal is not that migration. Source pin checks remain `make verify-issue-1393`.

Any `$1` receipt valuation is a display convention, not a redemption promise,
oracle feed, peg or enforceable collateral guarantee. Keep accounting in raw units.

Run `make verify-issue-1393`, then follow the checklist. Agents may record a
specific unavailable check as SKIP and report **PASS with skips** when every
executed check passes; never relabel a failed check or claim deployment/custody
acceptance from skipped checks. User instructions override workflow holds.
