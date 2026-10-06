# Route-local token tax protocol and migration (#1381)

[Issue #1381](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1381) replaces
wallet-funded router-hop taxes with funding from this transaction's swap amounts.
ALPHA is `terra1x6e64es6yhauhvs3prvpdg2gkqdtfru840wgnhs935x8axr7zxkqzysuxz`;
it is not canonical cLUNC. The existing ALPHA deployment code 11666 and catalog
pin 11630 are distinct. A catalog pin change does not upgrade funding semantics.

Invariants **RT1381-1–RT1381-10** and third-party agent instructions live in
[the playbook](../../skills/AGENTS_ROUTE_LOCAL_TAX.md). Review/acceptance evidence
and skip policy: [QA checklist](../qa/issue-1381/README.md).
Production automation has separate
[RTROLL1381-1–RTROLL1381-10](../../skills/AGENTS_ROUTE_TAX_ROLLOUT.md) and an
[agent rollout review checklist](../qa/issue-1381-rollout/README.md).

## Amount and identity protocol

Version **1** is an explicit token/router contract. Token `route_tax_capability`
returns `funding_version`, configured `router`, `buy_bps` and `sell_bps`.
`route_tax_preview { kind, amount, trader, pair }` returns
`funding_version/gross/economic/tax/net`. The preview is public information;
execute authenticates the router and its active `route_tax_context` independently.
That context binds trader, pair, offer/ask tokens, gross/economic amounts and
exact final/refund transfers. Merely supplying `Swap.trader` does not establish it.

For sell gross `G`, choose maximal `S` with
`S + floor(S*sell_bps/10000) <= G`. Send `S` to the pair, distribute the tax
according to the configured sinks, and refund any remaining integer residue to
the route payer. Gross 1,010,000 at 100 bps yields economic 1,000,000 and tax
10,000. Tax is based on the economic input, not an alternative gross percentage.
Checked integer arithmetic covers full raw Uint128 budgets; human decimals never
enter this calculation. No amount comes from a separate trader balance.

Routed buy tax is `floor(pair_output*buy_bps/10000)` at the actual pair→router
swap boundary. The router's received net amount becomes the next gross budget.
Tax is charged once even if this asset is sold on the next hop. Final delivery
and owned refunds use restricted `route_transfer` context, avoiding another buy
or incidental transfer tax. Non-swap liquidity/escrow/refund movements retain
their existing classification. Legacy direct-pair sell tax/SendFrom allowance
remains declared input plus extra debit; supported router entry instead accepts
the full authorized gross budget.

Simulations use the same previews. Router `config.funding_version=1` identifies
net output semantics; indexer/frontend must not subtract buy tax again. Reverse
simulation returns a sufficient gross offer and is checked with forward simulation.
Hybrid book/pool partitions must fit funded economic input. Version-1 exemptions use the authenticated trader’s per-token manager/directory
state; a router/pair protocol exemption cannot waive another trader’s tax. Legacy
direct-pair source/recipient classification remains separate. Manual hybrid reverse
queries rebuild the actual forward partition at the gross offer; greedy reverse
inverts the real capped book/pool engine. Existing trader controls and pair code-id
freeze checks continue to apply.

## Coordinated rollout

Use [`upgrade-1381-route-tax.sh`](../../scripts/upgrade-1381-route-tax.sh)
and its [engine](../../scripts/lib/route_tax_rollout.py) to prepare the inventory,
reviewed plan, stores, migrations, factory/launcher template rotation and affected
pair pin refresh. The [CLI transport](../../scripts/lib/route_tax_rollout_cli.py)
implements live queries and local SDK signing. The default planning/probing path
is read-only. Adding this script
to a PR does not run production transactions. Keep its manifest, journal and public
receipts in a directory outside the checkout, not in tracked evidence files.

Prepare and inspect the plan without signing:

```bash
bash scripts/upgrade-1381-route-tax.sh --help
# Requires the reviewed optimized artifacts and their checksums.txt.
bash scripts/upgrade-1381-route-tax.sh \
  --artifacts smartcontracts/artifacts \
  --checksums smartcontracts/artifacts/checksums.txt \
  --state-dir ~/.local/state/cl8y-dex/route-tax-1381
```

The defaults select Columbus-5 and the official factory/router/launcher/CMM and
ALPHA addresses. Use repeated `--token <address>` to add known affected instances
to discovery. Override `--node`/`--lcd` for working public endpoints. For an isolated
LocalTerra rehearsal, explicitly use `--chain-id localterra`, fixture addresses
through `--factory`/`--router`/`--launcher`/`--cmm`/`--governance`/`--alpha`, fixture endpoints
and the appropriate test keyring. Never reuse a production journal for a test chain.

Optional reviewed prerequisite artifacts use `--cmm-wasm <path>` with
`--cmm-sha256 <reviewed-hash>`, or `--launcher-wasm <path>` with
`--launcher-sha256 <reviewed-hash>`. This release targets launcher **1.0.0**;
its optional artifact migrates an older launcher to that version. A current
compatible 1.0.0 launcher needs configuration rotation only. Replacing a current
launcher with different wasm requires a properly versioned future release and
updated reviewed target; the script cannot force same-version migration.
Follow `--help` for complete supported options.

For a live CMM below 0.2.2, set `ROLL1381_CMM_WASM` to the reviewed treasury
artifact path and `ROLL1381_CMM_SHA256` to its checksum from the reviewed release
record, then include both arguments when planning and on every subsequent mode:

```bash
bash scripts/upgrade-1381-route-tax.sh \
  --artifacts smartcontracts/artifacts \
  --checksums smartcontracts/artifacts/checksums.txt \
  --cmm-wasm "$ROLL1381_CMM_WASM" \
  --cmm-sha256 "$ROLL1381_CMM_SHA256" \
  --state-dir ~/.local/state/cl8y-dex/route-tax-1381
```

This example remains read-only. Append `--execute` only when the operator is
actually performing the reviewed rollout, then use the same CMM inputs for
`--verify` and `--finalize`. The script stores/migrates the prerequisite treasury
through its inventoried administrator before dispatching owned-token migrations.

Only the operator runs the mutation modes after reviewing the immutable plan:

```bash
# Actual transactions: local keyring prompts and 2-of-3 signatures.
bash scripts/upgrade-1381-route-tax.sh --execute \
  --state-dir ~/.local/state/cl8y-dex/route-tax-1381
# Read-only confirmation after the contract rollout.
bash scripts/upgrade-1381-route-tax.sh --verify \
  --state-dir ~/.local/state/cl8y-dex/route-tax-1381
# After deployment.env has been applied and application/live checks pass:
bash scripts/upgrade-1381-route-tax.sh --finalize \
  --state-dir ~/.local/state/cl8y-dex/route-tax-1381
```

Every invocation uses the same artifacts and address/key overrides as planning;
changed inputs must not silently reuse the journal. An uncertain submitted hash
normally stops resume for reconciliation. `--rebroadcast-pending` explicitly permits
only the exact checkpointed signed bytes, never a new transaction with a new
sequence. `--max-total-fee-uluna` bounds cumulative transaction fees (default
5,000,000,000 uluna); review the budget for the actual fleet before execution.
`deployment.env` contains public application pins/registries for the handoff,
not secret credentials. This PR's source review does not run any production mode.

The rollout retains official contract addresses. It targets router **1.4.0**, community
tax tokens **1.3.0**, factory **1.12.0**, pair **1.21.0**, and launcher **1.0.0**.
Production factory 1.10.0 has a different stored hash from the repository artifact;
The integrated factory 1.12.0 release also includes bounded hook admission
(#1398); its version permits the reviewed upgrade. Review the exact deployed artifact hash rather
than inferring wasm equivalence from a shared version label. The
[read-only baseline and release targets](../qa/issue-1381-rollout/README.md#release-targets-and-read-only-baseline)
record the observed code IDs; re-probe them before a new production plan.
Factory/pair migrations are prerequisites only when their live versions are behind
these reviewed artifacts. The launcher can retain its current compatible wasm and
rotate `token_code_id` through configuration; it must preserve AutoLP configuration.
A same-version deployment with a different hash is not a safe skip and cannot be
forced through the contracts' increasing-version migration guards. Resolve it with
a reviewed, properly versioned release before the corresponding mutation.

Inventory includes all factory pair CW20 assets, the launcher template pin and its
deployed code instances, and explicit affected tokens such as ALPHA. Include KENA
and other community-tax instances with zero current tax rates when bound to the
official factory/router; they can enable tax later and still need compatible code.
Reject unexpected bindings/admins. Token symbols and catalog pins do not establish
code provenance or funding capability.

The supplied local keys have distinct roles:

| Key | Role |
|---|---|
| `cl8ydeploy` | Store reviewed wasm; its public address is `terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv`. |
| `multisig_2of3` | Treasury/governance custody where live ownership proves it; address `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7`. |
| `multisig1`, `multisig2` | Default two component signatures; `multisig3` is the alternative third component, not an additional administrator. |
| `cl8y2_admin` | Optional direct EOA migrations only for contracts whose actual admin is `terra1xsecn4snv94ezcez0z3vq8an9j4h4kxxcydp8l`. |

The operator enters the keyring passphrase once; the process retains it only in
memory. ALPHA's admin is CMM, so its migration is a CMM `migrate_owned_contract`
execution through the actual CMM owner/governance dispatcher. Prove compatible
CMM dispatch support (**>=0.2.2**) before attempting it. If missing, include the
reviewed CMM prerequisite artifact and upgrade its real administrator first.
An available EOA key cannot bypass contract-admin custody.

Execution journals submitted transaction hashes and inclusion receipts. If a
terminal or network failure occurs, resume the same manifest/journal and reconcile
pending transaction hashes before any resend. Do not create a new plan to conceal
an uncertain transaction. Pauses introduced during this transition stay in place
on a partial failure; existing pauses remain preserved. Explicit finalization
restores only markets this rollout paused, after application rollout and live
checks described below.

1. Build/test the exact revision, generate schemas, record router/token wasm hashes
   and run `make verify-issue-1381`. Use isolated LocalTerra fixtures first.
2. Inventory live router, token instances, administrators, configured factory/router,
   code IDs/hashes, registered pairs, rates and exemption directories. Keep the
   Create/Manage catalog provenance and pin policy separate from trading capability.
3. Store the reviewed artifacts using the existing authorized release facilities.
   Upgrade prerequisite factory/pair versions and CMM dispatch support where
   needed. Migrate the official router and community-tax instances together.
   ALPHA's admin is CMM: use governance `migrate_owned_contract`, not an EOA
   `wasm migrate` pretending to own ALPHA. Existing tooling is
   [`upgrade-1246-alpha.sh`](../../scripts/upgrade-1246-alpha.sh) and its treasury
   governance helpers; select the new reviewed code, never reuse 11666 as v1 proof.
4. Whitelist only the proven new tax-token wasm in the factory, preserve required
   old whitelist entries during migration, rotate factory `pair_code_id` and
   launcher `token_code_id` to reviewed templates without changing AutoLP settings,
   and refresh affected pair asset code IDs through factory
   `refresh_pair_asset_code_ids`/batch after token migration. Verify pauses,
   registration, live pins and exact tax sinks before resuming swaps.
5. Use the emitted public deployment environment snippets to configure indexer
   `TRADING_ROUTE_TAX_CODE_IDS`/`TRADING_ROUTE_TAX_DATA_HASHES` and reviewed
   `TRADING_TAX_FREE_CODE_IDS` from actual stores. Align Create/Manage catalog pins
   with the new launcher template while retaining independent provenance checks;
   restart/invalidate old route caches and refresh independent live capabilities.
   Roll out indexer and frontend together so funded net quotes and gross input
   submission use version 1. An unsupported legacy tax route stays explicitly
   unavailable; it cannot fall back to wallet extra debit.
6. Run read-only code/hash/config/capability/preview probes while affected markets
   remain paused. After the application rollout and these checks, explicitly
   finalize to restore only markets that were unpaused before this rollout.
   Immediately conduct an authorized bounded execution smoke with an independently
   funded synthetic initial-offer account and zero intermediate balances. Compare
   quote, sink/supply ledger, pair credit, final net output, refunds and gas against
   the recorded input. Re-pause affected markets if the live smoke fails and
   investigate before further trades. Paused markets cannot execute that smoke;
   read-only previews alone do not establish execution success. Record public
   migration/smoke receipts against AC9.

Probe example (no signing):

```bash
python3 scripts/qa/probe-route-tax-1381.py --expect-funded \
  --router <official-router> --tokens <migrated-alpha> \
  --pair <alpha-sell-pair> --trader <synthetic-trader>
```

Use `bao-exec` from openbao-tools to inject only the required inventoried API
credential into the existing authenticated release/Forgejo child process. Never
print it or put credentials in receipts. Unavailable signing custody is an
explicit skipped deployment check, not a reason to hold implementation/PR work.

Rollback must restore a compatible router/token/indexer/frontend set and invalidate
cached v1 routes. Do not re-enable a wallet-funded taxed intermediate route while
the client still promises a gross-only budget. Keep unsupported-version exclusion
until a verified coordinated deployment exists. Source implementation, test-chain
execution and Columbus-5 deployment are distinct evidence.
The script does not promise automatic downgrade: increasing-version guards and
storage compatibility can reject a previously stored wasm. Follow
[SEC-H05 rollback limitations](wasm-admin-migration.md#rollback-and-limitations-sec-h05),
keep the partial fleet contained and prepare a reviewed forward repair when a
downgrade is unsupported.

## Launch-guard refinement (#1390)

Router and community-tax-token **1.3.0** retain funding version **1** and introduce
[LG1390](../../skills/AGENTS_ROUTE_LAUNCH_GUARDS.md). Update both contracts in the
coordinated migration inventory above, refresh factory whitelist/pair code-id
pins and capability caches, then verify actual code hashes and cw2 versions.
The router rejects repeated capable assets and publishes an optional authenticated
original-input refund label. The token uses that label only to restore the
payer's original integer residue above a lowered cap; old contexts without it
retain cap enforcement. Acquired output, intermediate residue and other
recipients remain capped. Public Sell/Buy previews reject protocol-exempt traders.

Follow [the agent review and test-chain checklist](../qa/issue-1390/README.md).
An implementation PR or local receipt does not establish production deployment;
record unavailable live work as an explicit skip.

Router/token 1.3.0 trust recovery and single-use settlement: follow the
[router trust runbook](router-trust-1388.md) and
[agent checklist](../qa/issue-1388/README.md). Immediate rollout must preserve the
router address and reject every inventoried stored binding mismatch before mutation.

## Funded reverse provider gas validation (#1422)

Router **1.4.0** reuses immutable query-local capability/pair lookup metadata;
all amount-specific previews and fresh final sufficiency validation remain real
queries. Pair **1.21.0** remains the reviewed bounded-reverse target. These targets
supersede old router 1.3.0/pair 1.19.0 rollout plans in
[#1384](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1384) and
[#1386](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1386).
Run [the six funded 3M fixtures](../qa/issue-1422/README.md) and
`make verify-issue-1422`, preserving actual smart responses/errors, optimized
code checksums, cw2, node image/config and query-probe receipts. Wrapper receipt
gas includes transaction overhead and is not the smart-query meter reading.

Before production rollout acceptance, probe the actual selected provider
read-only for matching deployed hashes/capabilities and funded forward/reverse
quotes at its documented real limit. Record provider, limit, route/state and
responses; no source simulation-count bound or larger-limit local test proves
availability there. Include the DB exact-output client's fresh forward preflight
and direct router reverse as separate quote paths. Deterministic gas failures
retain Receive intent and invalidate executable quotes; no endpoint fanout,
automatic retry, weaker signing protection or tax waiver is permitted. Keep
unavailable production acceptance explicitly unverified while source review may
PASS with explicit skips under [FRQ1422](../../skills/AGENTS_FUNDED_REVERSE_QUERY_BUDGET.md).
