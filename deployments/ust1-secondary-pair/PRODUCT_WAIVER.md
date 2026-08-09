# Product waiver — UST1 secondary AMM pair (GitLab #508 Path B)

**Status:** Interim waiver (Path B) while Path A tooling is ready.  
**Date:** 2026-08-08  
**Tracks:** [#508](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/508), parent [#502](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/502)

## Decision

Defer **live columbus-5** create + seed of UST1/vFDUSD (and UST1/cUSTC) until seed inventory exists. Do **not** create an unseeded or dust-only pool that could be mistaken for peg defense.

## Rationale

1. On 2026-08-08, factory queries show **no** UST1/vFDUSD or UST1/cUSTC pair.
2. UST1 CW20 `total_supply` is **0** — no mint inventory for LP seeding.
3. Soft-launch deployer `terra1hu4zgg…` holds **0** UST1 / vFDUSD / cUSTC (LUNC only for gas/fees).
4. Issue guardrails forbid empty/misleading markets and require documented seed amounts (invariant **U4**).
5. Path A scripts + LocalTerra fixture are shipped so ops can execute immediately when inventory exists (`DRY_RUN=1 ./scripts/add-ust1-secondary-pair.sh`).

## What is deferred

- Production `create_pair` + `provide_liquidity` for UST1/vFDUSD and/or UST1/cUSTC
- Production indexer listing / Trade-Swap smoke on those pairs
- Optional `set_discount_registry` on the new pair(s)

## What is not deferred

- Oracle window mint/redeem primary venue (`/ust1`, [#506](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/506))
- Wrap-mapper wiring (already on router per #502)
- Tooling, docs, invariants **U1–U7**, and LocalTerra rehearsal path in this repo

## Revisit trigger

Lift this waiver and run Path A when **all** are true:

1. `token_info` / balances: operator wallet holds UST1 **and** quote leg (vFDUSD or cUSTC) ≥ chosen seed (default 1e6 raw each), typically after window mint.
2. `total_supply(UST1) > 0` (or explicit decision to seed from a funded treasury).
3. Operator unlocks `cl8ydeploy` (or designated LP wallet) and runs `./scripts/add-ust1-secondary-pair.sh`.
4. Record pair address + create/seed txs in `addresses.env` / `deploy-trace.md` and comment on #508 + #502.

## Copy constraint (still in force)

No UI or docs may present AMM as UST1 mint/redeem (**U1**). Use secondary-market wording from `frontend-dapp/src/utils/ust1SecondaryMarket.ts` when linking Trade/Swap from `/ust1`.
