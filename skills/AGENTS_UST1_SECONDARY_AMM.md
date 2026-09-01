# Agent playbook: UST1 secondary AMM pair (GitLab #508)

Use when creating/seeding **UST1/vFDUSD** or **UST1/cUSTC** on the live CL8Y factory, rehearsing on LocalTerra, recording a **product waiver**, or changing `/ust1` / Trade/Swap copy that mentions secondary markets.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [`docs/runbooks/ust1-secondary-amm-pair.md`](../docs/runbooks/ust1-secondary-amm-pair.md) | Operator runbook + invariants **U1–U7** |
| [`scripts/lib/ust1-secondary-pair-defaults.sh`](../scripts/lib/ust1-secondary-pair-defaults.sh) | Mainnet anchors, seed defaults, pair leg |
| [`scripts/add-ust1-secondary-pair.sh`](../scripts/add-ust1-secondary-pair.sh) | columbus-5 create + seed (`DRY_RUN=1` supported) |
| [`scripts/seed-ust1-secondary-pair-local.sh`](../scripts/seed-ust1-secondary-pair-local.sh) | LocalTerra stand-in tokens + pair + LP |
| [`deployments/ust1-secondary-pair/`](../deployments/ust1-secondary-pair/) | addresses, deploy-trace, product waiver |
| [`frontend-dapp/src/utils/ust1SecondaryMarket.ts`](../frontend-dapp/src/utils/ust1SecondaryMarket.ts) | Secondary-market copy + `/trade/:pairAddr` + Swap `/?from=&to=` helpers (**U1**, [#711](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/711)) |
| Parent [#502](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/502) · child [#508](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/508) | Phase 4 AC |

## Rules of thumb

1. **Window first (U1)** — mint/redeem is `/ust1` → ust1-window, never router/AMM. Secondary-market CTA language: “secondary market” / “Trade or Swap”, not “mint”.
2. **Do not edit soft-launch gemstone catalogs (U6)** — never add UST1 to `mainnet-soft-launch-defaults.sh` `MAINNET_SOFT_LAUNCH_PAIRS`.
3. **Whitelist (U2)** — UST1/vFDUSD/cUSTC are code **10184** on columbus-5; still verify before create. New code IDs need governance + [`cw20-whitelist-policy.md`](../docs/runbooks/cw20-whitelist-policy.md). Post-listing pin: [`AGENTS_CW20_CODE_ID_PIN.md`](./AGENTS_CW20_CODE_ID_PIN.md) (**F6** / [#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582)).
4. **No native legs (U3)** — use cUSTC, not `uusd`.
5. **#518 upgrade must be live** — `create_pair` for UST1 keeps digits in the LP ticker (`UST1-CUST-LP`) and reverts on classic LP CW20 until factory `lp_token_code_id` is digit-allowing `cw20-mintable`. Run [`scripts/upgrade-518-lp-symbol.sh`](../scripts/upgrade-518-lp-symbol.sh). See [`AGENTS_LP_SYMBOL_DIGITS.md`](./AGENTS_LP_SYMBOL_DIGITS.md).
6. **Seed size honesty (U4)** — default smoke seed is **1.0** raw-unit human (1e6) per side on mainnet script; document any larger size. Empty pools are worse than a waiver. `UST1_SEC_SKIP_LP=1` requires `UST1_SEC_ALLOW_UNSEEDED=1`. Preflight asserts on-chain CW20 **symbols** match expected legs.
7. **Indexer (U5)** — factory `create_pair` is enough for discovery; foreign pairs fail provenance (**P1**). Indexer **Price (USD)** of 1 UST1 is ~$1 after [#522](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/522) — see [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md). The **dApp** defaults `/trade` + `/charts` to the **other token’s** USD via UI invert ([#524](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/524)) — [`AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](./AGENTS_FRONTEND_TRADE_PAIR_INVERT.md). Pair pickers list UST1 markets together ahead of faucet gems ([#534](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/534)) — [`AGENTS_FRONTEND_PAIR_CATALOG_RANK.md`](./AGENTS_FRONTEND_PAIR_CATALOG_RANK.md).
8. **Close AC (U7)** — either live seeded pair + txs on #508, or explicit waiver text on #508 **and** #502. Tooling-only PRs do not satisfy Path A.
9. **Keys** — host terrad via [`terrad-host.sh`](../scripts/lib/terrad-host.sh); never commit `TERRAD_HOST_KEYRING_PASS`. See [`AGENTS_KEY_CUSTODY.md`](./AGENTS_KEY_CUSTODY.md).

## Quick commands

```bash
make verify-issue-508
DRY_RUN=1 ./scripts/add-ust1-secondary-pair.sh
./scripts/seed-ust1-secondary-pair-local.sh   # needs LocalTerra + .env.local
VERIFY508_LOCAL=1 make verify-issue-508
VERIFY508_MAINNET=1 make verify-issue-508     # read-only pair presence
```

## Related

- [`AGENTS_MAINNET_SOFT_LAUNCH.md`](./AGENTS_MAINNET_SOFT_LAUNCH.md) — SL1–SL7 boundary; do not fold economic pairs into soft-launch
- [`AGENTS_FACTORY_ADDRESS_GUARD.md`](./AGENTS_FACTORY_ADDRESS_GUARD.md) — indexer factory provenance
- [`AGENTS_FEE_DISCOUNT_TIERS.md`](./AGENTS_FEE_DISCOUNT_TIERS.md) — `set_discount_registry` after create is belt-and-suspenders once factory **1.8.0** + pair **1.14.0** snapshot the factory pointer ([#536](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/536)); see [`AGENTS_FACTORY_DISCOUNT_REGISTRY.md`](./AGENTS_FACTORY_DISCOUNT_REGISTRY.md)
- [`AGENTS_DEPLOY_TRACE.md`](./AGENTS_DEPLOY_TRACE.md) — audit record pattern
- [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) — retail copy; keep secondary-market wording short
- `/ust1` UI track: [#506](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/506)
- Swap / Trade acquire must Guide to `/ust1`, never “Swap vFDUSD to mint” ([#678](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/678)): [`AGENTS_FRONTEND_SWAP_ACQUIRE_GUIDANCE.md`](./AGENTS_FRONTEND_SWAP_ACQUIRE_GUIDANCE.md)
- Swap AMM deep links (`ust1SecondarySwapPath` → `/?from=UST1&to=vFDUSD`): [`AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md`](./AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md) ([#711](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/711) / [#715](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/715))
- Pair Price (USD) for UST1/cUSTC and UST1/USTR: [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md) ([#522](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/522))
- Oracle rebalance + $5k LP to CMM: [`AGENTS_REBALANCE_MINT_UST1_LP.md`](./AGENTS_REBALANCE_MINT_UST1_LP.md)
- Trade/Charts other-side display invert: [`AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](./AGENTS_FRONTEND_TRADE_PAIR_INVERT.md) ([#524](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/524))
- Create Pair listed-CW20 picker (keep the U1 AMM ≠ mint notice): [`AGENTS_FRONTEND_CREATE_PAIR_PICKER.md`](./AGENTS_FRONTEND_CREATE_PAIR_PICKER.md) ([#542](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/542))
