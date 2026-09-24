# Agent playbook: pair TWAP Uint256 migration — Columbus-5 (#1324)

**Audience:** third-party agents asked to verify the production pair TWAP
migration, the indexer, or `/charts` after the cumulative-width release.

**Canonical runbook:**
[`docs/runbooks/pair-twap-uint256-columbus5.md`](../docs/runbooks/pair-twap-uint256-columbus5.md)
**Contract invariant:**
[`AGENTS_TWAP_CUMULATIVE_U256.md`](AGENTS_TWAP_CUMULATIVE_U256.md)
**Chart invariant:**
[`AGENTS_FRONTEND_PRICE_CHART.md`](AGENTS_FRONTEND_PRICE_CHART.md)

## Agent boundary

- `make verify-issue-1324` runs local guards. It does not prove mainnet state.
- `UPGRADE1324_PROBE_ONLY=1 ./scripts/upgrade-1324-pair-twap.sh` is the
  read-only live probe. It checks factory `pair_code_id`, every listed pair's
  code ID and cw2 version, the stored UST1/USTR observation ring, and
  `Observe([0, 60])`. It does not sign or mutate chain state.
- The default upgrade script stores wasm, updates factory configuration, and
  submits pair migrations. Do not run it as an unattended verification step.
  Store identity/keyring access and 2-of-3 signing belong to the human
  operator. If those actions are still required, create one cl8y-pm inbox card
  with the DEX issue URL; do not describe the ticket as verified or close it.
- A read-only swap/indexer record can prove reserve movement only when it is
  after the migration and contains a successful non-zero swap or LP event.
  `/gt/events` `reserves` are post-event reserves; compare distinct heights.
- Indexer and dApp are separate Coolify deploys. A healthy container by itself
  is insufficient: both deployment commits must descend from `c17e71d3`, the
  evidence endpoint must respond, and ALPHA candles must carry `usd_leg` and
  the `subject_*` fields consumed by the chart bundle.

## OpenBao use

For read-only Coolify metadata, retrieve only the selected variables at process
launch:

```bash
bao-exec --only COOLIFY_TOKEN,COOLIFY_URL -- python3 <read-only-coolify-check.py>
```

Never echo values, include them in command arguments, write them to files, or
request chain-signing material for this verification. The OpenBao wrapper is
documented in the [workstation guide](https://git.cl8y.com/PlasticDigits/openbao-ops/src/branch/main/docs/workstation.md).

## Cross-system references

- Pair storage and migration: `smartcontracts/contracts/pair/src/{contract.rs,state.rs,msg.rs}`.
- Indexer candle API and evidence API: `indexer/src/api/{pairs.rs,evidence.rs}`.
- ALPHA chart projection: [ADR 0012](../docs/adr/0012-alpha-pair-price-candles.md).
- GeckoTerminal post-event reserves: [`AGENTS_INDEXER_GT_EVENT_RESERVES.md`](AGENTS_INDEXER_GT_EVENT_RESERVES.md).
- Open issue/implementation chain: [#1324](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1324), [#1322](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1322), [merged PR #1323](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1323), and [#1315](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1315).
