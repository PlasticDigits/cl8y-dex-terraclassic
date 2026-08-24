# Agent playbook: tax-aware LocalTerra swarm (GitLab #621)

Use when changing **localnet swarm** pair picking, CW20 `Send` sizing, or workers that might hit the listed community-tax token.

Sibling seed/funding is [#620](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/620). Do **not** reopen swarm liquidity ([#293](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/293)) unless OE-1 gem hub checks regress. Do **not** turn hybrid off to “make tax apply” ([#596](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/596) / **H-01**).

Parent classify: [`AGENTS_COMMUNITY_TAX_ROUTER.md`](./AGENTS_COMMUNITY_TAX_ROUTER.md) (**T592-13** / **R607**). General swarm: [`AGENTS_LOCALNET_TRADING_SWARM.md`](./AGENTS_LOCALNET_TRADING_SWARM.md).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#621**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/621) | Tax-aware swarm workers |
| [`taxDetect.ts`](../packages/localnet-trading-swarm/src/taxDetect.ts) / [`taxPreview.ts`](../packages/localnet-trading-swarm/src/taxPreview.ts) / [`pairPick.ts`](../packages/localnet-trading-swarm/src/pairPick.ts) / [`taxHooks.ts`](../packages/localnet-trading-swarm/src/taxHooks.ts) | TS detect, debit math, exclude, hooks |
| [`swarm_tax.py`](../scripts/bots/swarm_tax.py) | Python detect / debit / exclude (no chain) |
| [`actions.ts`](../packages/localnet-trading-swarm/src/actions.ts) | Tax-listed actions + `tax_hybrid_skip` |
| [`swarm.py`](../scripts/bots/swarm.py) `--worker tax` | Python tax worker |
| [`launch-swarm.sh`](../scripts/bots/launch-swarm.sh) | Starts `tax-0` unless `SWARM_TAX_WORKERS=0` |
| [`packages/localnet-trading-swarm/README.md`](../packages/localnet-trading-swarm/README.md) | Package invariants |

## Invariants **S621-1–S621-8**

1. **S621-1 — gem exclude.** Default gem / OE-1 workers never offer a tax-token address and never use a pair that includes one for swap / router / hybrid / LP. Same idea as wrap Mint skip. Limit workers **may** place 1:1 on tax/EMBER (**T592-7**).
2. **S621-2 — dedicated tax workers.** Default on (`SWARM_TAX_WORKERS` unset/`1`). TypeScript wallet 4 is profile `tax_listed` (tax/EMBER + official-router ≥2hop). Python `launch-swarm.sh` starts one `--worker tax` **before** gem workers (leftover #625 soak). First cycle is `hybrid` skip then pair `sell` so `tax_listed` / `tax_debit` / `tax_hybrid_skip` appear without waiting on RNG. `SWARM_TAX_WORKERS=0` is exclude-only (no tax volume).
3. **S621-3 — sell extra-debit.** Before a tax-token `Send` swap, query `TaxPreview`. `Send.amount` stays the **economic** amount (pair still credits `amount`). Balance gate uses preview `debit` (pair-direct) or `debit + hop_trader_debit` (router hop). Missing preview → `amount * (1 + sell_bps/10000)` fail-closed. Never broadcast a 100% balance sell.
4. **S621-4 — trader field.** Pair-direct `Swap.trader` stays unset (token extra-debits `from`; spoof ignored). Official-router ≥2hop `Send`s to `config.router` / `VITE_ROUTER_ADDRESS`; the router stamps `Swap.trader` (**T592-13**). Bots must not set `trader` on pair-direct.
5. **S621-5 — 1:1 paths.** Provide / withdraw / place-limit stay 1:1. Do not add tax to limit `Send`.
6. **S621-6 — hybrid.** Do not broadcast a 1:1-sized hybrid sell on the tax pair. Log `tax_hybrid_skip`. Do not disable hybrid globally (**#596**).
7. **S621-7 — OE-1 hubs stay gems.** Do not add the tax pair to OE-1 reciprocal `pool_only` symmetry checks until quotes are net-of-tax. Hub deepen stays EMBER/CORAL, TOPAZ/ONYX, ONYX/CORAL.
8. **S621-8 — not a FoT gas model.** Router+tax uses the same hop padding / `SWAP_GAS_BUFFER` as the dApp ([#115](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/115)). Prefer Transfer top-up from #620, not Mint. Shared `test1` + five TS wallets must stay above the #372 soak floor.

## Structured logs

TS JSON lines and Python tax-worker prints include `tax_debit`, `tax_credit`, `bps`, `path=pair|router` so QA can grep extra-debit vs 1:1. Hybrid skip is `note=tax_hybrid_skip` / `[tax-0] tax_hybrid_skip`.

## Verify

```bash
make verify-issue-621
cd packages/localnet-trading-swarm && npm run test:run
make test-swarm-liquidity
make verify-issue-293     # gem hub OE-1 still pool_only
# after seed deploy:
# ./scripts/localnet-trading-swarm.sh -- --dry-run
# make swarm-launch && make swarm-stop
# leftover live after !415 (soak + OE-1 293):
# make verify-issue-625
```

## Do not

- Point gem random walks at the tax pair (underfunded extra-debit / silent skip).
- Spoof `Swap.trader` on pair-direct.
- Size hybrid book+pool sells as 1:1 on the tax token.
- Add the tax pair to OE-1 hub symmetry.
- Enable MintControl on the QA tax token so funding can Mint ([#620](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/620) **L620-2**).
- Turn hybrid off for `code_id` 11611 (**R607-5**).

Leftover live soak after !415: [`AGENTS_POST_MERGE_OPS_625.md`](./AGENTS_POST_MERGE_OPS_625.md) ([#625](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/625)).
