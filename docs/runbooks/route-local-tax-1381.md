# Route-local token tax protocol and migration (#1381)

[Issue #1381](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1381) replaces
wallet-funded router-hop taxes with funding from this transaction's swap amounts.
ALPHA is `terra1x6e64es6yhauhvs3prvpdg2gkqdtfru840wgnhs935x8axr7zxkqzysuxz`;
it is not canonical cLUNC. The existing ALPHA deployment code 11666 and catalog
pin 11630 are distinct. A catalog pin change does not upgrade funding semantics.

Invariants **RT1381-1–RT1381-10** and third-party agent instructions live in
[the playbook](../../skills/AGENTS_ROUTE_LOCAL_TAX.md). Review/acceptance evidence
and skip policy: [QA checklist](../qa/issue-1381/README.md).

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

1. Build/test the exact revision, generate schemas, record router/token wasm hashes
   and run `make verify-issue-1381`. Use isolated LocalTerra fixtures first.
2. Inventory live router, token instances, administrators, configured factory/router,
   code IDs/hashes, registered pairs, rates and exemption directories. Keep the
   Create/Manage catalog provenance and pin policy separate from trading capability.
3. Store the reviewed router/token artifacts using the existing authorized release
   facilities. Migrate the official router and community-tax instances together.
   ALPHA's admin is CMM: use governance `migrate_owned_contract`, not an EOA
   `wasm migrate` pretending to own ALPHA. Existing tooling is
   [`upgrade-1246-alpha.sh`](../../scripts/upgrade-1246-alpha.sh) and its treasury
   governance helpers; select the new reviewed code, never reuse 11666 as v1 proof.
4. Whitelist only the proven new tax-token wasm in the factory, preserve old pins
   during migration, and refresh affected pair asset code IDs through factory
   `refresh_pair_asset_code_ids`/batch after token migration. Verify pauses,
   registration, live pins and exact tax sinks before resuming swaps.
5. Configure indexer trusted route-tax code/hash identities from actual stores;
   restart/invalidate old route caches and refresh independent live capabilities.
   Roll out indexer and frontend together so funded net quotes and gross input
   submission use version 1. An unsupported legacy tax route stays explicitly
   unavailable; it cannot fall back to wallet extra debit.
6. Run read-only code/hash/config/capability/preview probes, then bounded execution
   smoke with an independently funded synthetic initial-offer account and zero
   intermediate balances. Compare quote, sink/supply ledger, pair credit, final
   net output, refunds and gas against the recorded input. Read-only previews alone
   do not establish execution success.

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
