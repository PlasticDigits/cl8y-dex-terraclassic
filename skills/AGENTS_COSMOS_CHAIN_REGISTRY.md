# Agent playbook: cosmos chain-registry (#642 / upstream #7910)

Use when maintaining the Terra Classic CW20 listing in
[cosmos/chain-registry#7910](https://github.com/cosmos/chain-registry/pull/7910).

Pack: [docs/listings/cosmos-chain-registry](../docs/listings/cosmos-chain-registry/README.md).
Evidence: [docs/qa/issue-642](../docs/qa/issue-642/README.md).
Verify: `make verify-issue-642`.

1. Use `terra/` (columbus-5). Pin the permanent six to #639's catalog. Only the
   exact #1260 USDT contract is an exception; no gems or community-tax templates.
2. Bridge traces must reference registered origins. BSC CL8Y has 18 decimals,
   Venus vFDUSD has 8, and the Terra vFDUSD representation has 6. Preserve the
   bridge's actual decimal conversion. Keep `ceramicliberty-com` on BSC only.
3. cLUNC/cUSTC trace to same-chain `uluna`/`uusd`. Query both `minter` and
   `minters`; an EOA primary minter does not exclude an additional wrap mapper.
4. CL8Y includes a 500,000-token Terra launch allocation. Use contract history
   and explicit bridge-mode queries before comparing total supply to escrow.
   Do not call the whole supply backed by the current BSC LockUnlock balance.
5. USDT includes Bridge-Debt local issuance. Disclose that fact and omit a
   Terra CoinGecko ID. This pack fulfills only #1260's chain-registry portion.
6. Put new images at their origin and use `image_sync` for bridged entries.
   Reuse upstream's existing USDT images; preserve the DEX logo in the offline pack.
7. Use `gh` for upstream GitHub work and `fj` for git.cl8y.com tracking.
   Run upstream schema, JSON lint, and data checks before pushing. Leave merge
   and downstream propagation pending until observed; a submitted PR is not
   evidence that Leap/Directory already displays the assets.
