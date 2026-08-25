# Agent playbook: AutoLP factory pair + skim floor (GitLab #610)

Use when changing the **community-tax AutoLP** sister, launcher AutoLP instantiate, Manage Token pair copy, or skim execution.

Parent template [#592](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592) ([`AGENTS_COMMUNITY_TAX_CW20.md`](./AGENTS_COMMUNITY_TAX_CW20.md) **T592-10**). LocalTerra seed binds AutoLP `pair` after factory list ([#620](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/620), [`AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md`](./AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md) **L620-4**). Post-merge leftover LCD pair + AutoLP bind: [#624](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/624) ([`AGENTS_POST_MERGE_OPS_624.md`](./AGENTS_POST_MERGE_OPS_624.md) **M624-2**). Bind / SKU init [#605](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/605) (**H-1** — AutoLP is paid and bound at create; **do not re-do bind here**). Audit **M-2 / M-3** in [`INTERNAL_KIMIK3_1787468843`](../audits/INTERNAL_KIMIK3_1787468843.md).

Product decision (2026-08-23): `pair` must be factory-listed **and** have the tax token as one side.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#610**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/610) | Factory pair + skim floor |
| [`community-tax-autolp`](../smartcontracts/contracts/community-tax-autolp/) `pair.rs` / `spread.rs` | Lookup + floor |
| [`docs/contracts-terraclassic.md` § AutoLP](../docs/contracts-terraclassic.md#community-tax-cw20-gitlab-592) | Message shapes |
| [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) **M610-1** | Invariant table |
| [`AGENTS_COMMUNITY_TAX_CW20.md`](./AGENTS_COMMUNITY_TAX_CW20.md) | **T592-10** |
| [`AGENTS_FRONTEND_CREATE_TOKEN.md`](./AGENTS_FRONTEND_CREATE_TOKEN.md) | Manage Token copy (**C605-3**) |

## Invariants **M610-1–M610-8**

1. **M610-1 — factory-listed tax pair.** `pair` on instantiate or `UpdateConfig` must succeed `factory.Pair { asset_infos }` for the **immutable launcher factory** and include **`cfg.token` as one of the two `asset_infos`**. Store only the factory-returned `contract_addr`. Reject native-only / wrong-token / non-factory contracts. Same check as token `RegisterListedPair` (**T592-9**). Setting `pair` also executes `register_listed_pair` on the tax token ([#633](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/633) **R633-3** / [`AGENTS_COMMUNITY_TAX_AUTOREGISTER.md`](./AGENTS_COMMUNITY_TAX_AUTOREGISTER.md)).
2. **M610-2 — immutable factory.** Factory is stamped from the launcher at instantiate. Manager **cannot** `UpdateConfig` factory (closes fake-registry spoof).
3. **M610-3 — skim floor.** `SkimToLp` always sets `max_spread` (default **100 bps**, hard cap **200 bps**). Optional manager `skim_min_return`. Never leave both `None`. Permissionless caller cannot pass or loosen the floor. Manager cannot set `skim_max_spread` above 200 bps. `UpdateConfig { skim_min_return: 0 }` clears a leftover hostile absolute floor (seed-path `#625` / **C623-8**).
4. **M610-4 — floor revert keeps tax.** Swap that violates `max_spread` / `min_return` reverts the tx. Tax stays on AutoLP. `SKIMMING` rolls back with the tx.
5. **M610-5 — T592-10 unchanged.** `SkimToLp` stays permissionless and is **never** called from token `Transfer` / `Send` or pair `AfterSwap`. Do not add pair/router FoT math (**H-01**).
6. **M610-6 — merge.** Omitted `UpdateConfig` fields keep their previous values (including pair, factory, floor).
7. **M610-7 — reject on set.** Fake pair / wrong listed pair revert on instantiate or `UpdateConfig`, not only on skim. `poc_autolp_manager_can_skim_to_fake_pair` is inverted.
8. **M610-8 — reentrancy residual.** `SKIMMING` is true for the swap submsg (pair hook calling `SkimToLp` → `Reentrancy`). Reply clears the lock **before** provide messages. A listed pair's provide hook could call `SkimToLp` again in the same tx. Factory+token-side listing is what keeps that pair in the CL8Y set.

Live LocalTerra `SkimToLp` vs a factory pair (floor + fake-pair reject) is the named tax-on suite: [`AGENTS_CW20_CODE_ID_TAX_ON.md`](./AGENTS_CW20_CODE_ID_TAX_ON.md) / `make verify-issue-623` (**C623-8**). `#601` smoke still leaves AutoLP `null`.

## Verify

```bash
make verify-issue-610
make verify-issue-616
make verify-issue-620
make verify-issue-623
cd smartcontracts && cargo test -p cl8y-community-tax-autolp
cd smartcontracts && cargo test -p cl8y-community-token-launcher poc_autolp_manager_can_skim_to_fake_pair
```

Columbus-5 AutoLP **11621** is the live launcher pin (store was **11613**). No instances required until a token binds. Do not whitelist **11613** / **11621**. Post-merge leftover: [#616](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/616) ([`AGENTS_POST_MERGE_OPS_616.md`](./AGENTS_POST_MERGE_OPS_616.md)). Seed-path hostile `skim_min_return` leftover after !416: [#625](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/625) ([`AGENTS_POST_MERGE_OPS_625.md`](./AGENTS_POST_MERGE_OPS_625.md)).

## Do not

- Leave `max_spread` and `min_return` both `None` on the skim swap hook.
- Let the manager point `factory` at a mock registry.
- Call `SkimToLp` from taxed `Transfer` / `Send`.
- Treat a random CW20 as `pair`.
- Add pair/router FoT math.
