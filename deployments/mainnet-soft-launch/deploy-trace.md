# Deploy trace — columbus-5 — 2026-07-12 UTC

Filled from [`docs/templates/deploy-trace.md`](../../docs/templates/deploy-trace.md) for soft-launch (non-economic). Paste onto [#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391).

| Field | Value |
|-------|-------|
| **Operator** | answorld (`cl8ydeploy`) |
| **Governance multisig** | `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7` |
| **Network / chain ID** | columbus-5 |
| **Deploy type** | initial launch (mainnet soft launch — non-economic CW20) |
| **Git SHA** | `da811ea6763e94162d57294d34c6dc71d68c9338` |
| **Git describe** | `brouie-review-1238-gda811ea-dirty` (working tree had soft-launch script/docs fixes after store; **on-chain wasm hashes match local artifacts**) |
| **Terra Classic chain version** | terrad client `3.5.0-rc.0`; remote `node_info.version` `0.38.19`; sample height ~`29458706` |
| **RPC / LCD** | `https://terra-classic-rpc.publicnode.com:443` / `https://terra-classic-lcd.publicnode.com` |
| **Deployer** | `terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv` |
| **Addresses file** | `deployments/mainnet-soft-launch/addresses.env` |

### Contract code IDs

| Contract | Code ID | Address (if instantiated) | Store tx |
|----------|---------|---------------------------|----------|
| Factory | 11505 | `terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea` | `E268F6B8661461386FEE00F8E886E094BD73D8561D1BCAAB12DDF50352A6626E` |
| Pair | 11506 | (template; 10 instances below) | `DC60D2A137141689E8195272EA825C6A4079CF5A994E673B41CD4A476D937964` |
| Router | 11507 | `terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw` | `E56363CC04DD29FA52325F5035840A64065DFE1CEFCBE50F2FD20224A51C17FC` |
| Fee-discount | 11508 | `terra1wcczsdk7jwj99n3my6wx8wr4ee0hn6yaapgd792lgx5elrdtrn2scfnecz` | `8D11905C70E18ED2EBACFD08A5468DA4353DCB4FD33F587AF55C435382CE5998` |
| Soft-launch faucet (#473) | 11509 | `terra1388y0ppe2c3dy4nrmnpqp7e4ggukkrnmpzfjadfeu0pu2rm9cvkslfzcen` | `A9C19707E48BFD81BE55F2E07A67568636A77953E723B34219082AC3E5FAB879` |
| Hook(s) | — | none (pool-only soft launch) | — |
| CW20 base (reuse) | 6036 | QUARTZ + PEARL + LP tokens | Terraswap mainnet |
| CW20 mintable (reuse) | 10184 | EMBER…TOPAZ | PlasticDigits cw20-mintable |

**Canonical instantiate txs (bootstrap-governance run):** factory `69B1BAEF747D61900A27ED7D1DB6B786A24681F9D1AA6B2E143A047D3DAFE135`, router `F5EB88CDC31B78D1DAC77B3B2D838268DC86B7654BA4C15718FBCE735825C629`, fee-discount `9C7C2550B03596C598053C4A3C83E7E5B400610D60263642D4720A136F0FF543`.

**Handoff:** fee-discount `update_config` `C9F7D3751EA2190B26A14B4DA38645B2AC141D1612A084D44C9FA1A1BCFD1BC6`; factory `update_config` `D7689FA31B479937D39A2468BFE6BB1522202E19FAD4BD4137894ECFE6513640`. `set_discount_registry_all` `1695ADE7D5E5B7496708737E940BC8DA9723908DEB48C80FC720B915F65D0C1C`.

### Soft-launch tokens + pairs

| Symbol | Kind | Address |
|--------|------|---------|
| EMBER | mintable | `terra1dmuruhht32x8f47nvm73pwp6q7uf2jtfhdt3nxcql4mmqkyfsraqn2dt94` |
| CORAL | mintable | `terra1k6cqupylk0wp4pj273pntwhv9py0q5guyqye8ssvukn9xq7mes7sdlmena` |
| JADE | mintable | `terra1ejq3mjjgnklpa3pg4jterlfwsny055gpmcjf3fz0ev3ueajnzeysz6xxgr` |
| ONYX | mintable | `terra178fgrfzv7njtmdp9vghyf2dx77sah8u8jluzs7ym562chaxnmj2s6mn6m9` |
| RUBY | mintable | `terra1fga508hzx8dd7x8q4uhm6mdhkqv6fxrtsea3r27smdqmv5k2jgxq5zk9fc` |
| TOPAZ | mintable | `terra12k67cvfs7y7g8lca3qr4g4py6s6j69fu24gze5pjfamfpckv8mps7cymme` |
| QUARTZ | base | `terra17dpnjlpgsnm8muu4msfjra4f2hrptnjp2jdpkka4p0e3px42ayxq0pmc2z` |
| PEARL | base | `terra18fzufz8cs7ez49xjwgs248x85za5v50yug55fj7lyxp9hapxyr7qnh3czs` |

| Pair | Address | `total_share` (verified) |
|------|---------|--------------------------|
| EMBER/CORAL | `terra1klwuxas6x7p6fjde60kq70t0hu86wvt3fvyr2vgs0nn32fnv0q4qwznwp4` | `100000000000` |
| EMBER/JADE | `terra1y5xxv980jn0qu7n7y3slhtjehta6nlpqjkgcxl80uetdx84dxa4qegjhtx` | `10000000000` |
| EMBER/ONYX | `terra16827w2c7zcvetck9xz8d6ds3379v77gelwra6jdafqkx9q9u0r8qvkluu0` | `14142135623` |
| CORAL/RUBY | `terra1ra7cugjhchr45kdupxe2al5fna0zxu6syl8xhpanfk8dsvkq9lksf6fm9l` | `7071067811` |
| JADE/TOPAZ | `terra1nqjvd2xatac5ydcs6nstw7zp2yjc20p632ycxtevtf3rr2554fqswstx0n` | `100000000000` |
| ONYX/QUARTZ | `terra1havxdjfyphjazc342r3cj2n3kslsptac2eunvw8uzayusywg9t4shtuz7v` | `70710678118` |
| RUBY/PEARL | `terra1p0sd0t2ggm9ye43gp0ryadx3wwkz5hzn99hnz93ve99397xvuufsvsmw73` | `7071067811` |
| EMBER/QUARTZ | `terra1mp72n97rzwmqwudzycjj0e4jveetjnp622gnprv6ugqt3hfxg60sr5gkjm` | `50000000000` |
| CORAL/PEARL | `terra16k6huf87gzvnlgpvf85f8xfgawl6y2l5d4d5qdpyhknqaran9s5qx63c3r` | `70710678118` |
| JADE/ONYX | `terra1pc7dvcucrl9sr4r2nhr2rv3ywhthskqqtvtvaerx9cff7l8gesdskjgmn6` | `100000000000` |

### Wasm artifact hashes (`smartcontracts/artifacts/` — matches on-chain `data_hash`)

```
61f8a1956a4b6cb1702ba071b1a8c6cf8e499d6db627a77d828c8cccb39ce45d  cl8y_dex_factory.wasm
50ad70c579e4c376a90c103f4a5965baaa1d4957442b974aaa3fffd880943ba4  cl8y_dex_pair.wasm
f31a4aad4609572eea840c68a0afed20cca57b914c7cfb27d47bf22d51d19282  cl8y_dex_router.wasm
7ae5fd86fc77491f83db60d48aca8af79883f5edfe2d4de6832c45824fa8a4a9  cl8y_dex_fee_discount.wasm
```

On-chain code `data_hash` (LCD `/cosmwasm/wasm/v1/code/{id}`): **MATCH** for 11505–11508.

### Test results (pre-deploy evidence — SEC-H08)

| Suite | Command | Evidence |
|-------|---------|----------|
| Contracts | `make test-contracts` | **Not green on deploy SHA.** Pipeline [2670191691](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/pipelines/2670191691) failed `cargo-audit-smartcontracts` / `cargo-audit-indexer`; `test-contracts` / `test-frontend` / `test-indexer-integration` were **skipped**. Risk acceptance: soft-launch proceeds on artifact↔chain hash match + post-deploy verify. |
| Indexer integration | `make test-indexer-integration` | Skipped in same pipeline (see above). |
| Frontend unit | `make test-frontend` | Skipped in same pipeline (see above). |
| Soft-launch defaults | `make test-mainnet-soft-launch-defaults` | Local unit path for defaults/wiring (pre-deploy). |
| Fee-tier docs | `make check-fee-discount-tier-docs` | `OK: 11 tiers aligned` (verified 2026-07-12). |
| Pool swap smoke | `./scripts/smoke-pool-swap.sh` | See post-deploy below. |

```
Pipeline for da811ea: https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/pipelines/2670191691
status=failed (cargo-audit); test-* jobs skipped
```

### Post-deploy verification

```bash
TERRA_RPC_URL=https://terra-classic-rpc.publicnode.com:443 \
TERRA_LCD_URL=https://terra-classic-lcd.publicnode.com \
VERIFY_CONFIG_EXPECT_GOVERNANCE=terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7 \
VERIFY_CONFIG_EXPECT_TREASURY=terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7 \
VERIFY_CONFIG_EXPECT_DEFAULT_FEE_BPS=180 \
VERIFY_CONFIG_MIN_WHITELISTED_CODE_IDS=2 \
VERIFY_CONFIG_MIN_TIERS=11 \
  make qa-verify-deploy-config
# (addresses injected from deployments/mainnet-soft-launch/addresses.env into indexer/.env for the run)

PAIR_ADDR=terra1klwuxas6x7p6fjde60kq70t0hu86wvt3fvyr2vgs0nn32fnv0q4qwznwp4 \
OFFER_TOKEN=terra1dmuruhht32x8f47nvm73pwp6q7uf2jtfhdt3nxcql4mmqkyfsraqn2dt94 \
OFFER_AMOUNT=1000000 \
TERRA_LCD_URL=https://terra-classic-lcd.publicnode.com \
  ./scripts/smoke-pool-swap.sh
```

**`qa-verify-deploy-config` (2026-07-12T03:42:43Z) — RESULT: PASS (12 checks, 0 failures):**

```
=== CL8Y DEX post-deploy config verification (SEC-H03 / GitLab #441) ===
timestamp: 2026-07-12T03:42:43Z
git_sha: da811ea
lcd: https://terra-classic-lcd.publicnode.com
factory: terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea
router: terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw
fee_discount: terra1wcczsdk7jwj99n3my6wx8wr4ee0hn6yaapgd792lgx5elrdtrn2scfnecz

[1/6] Factory config (query: config)...
  [PASS] governance=terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7
  [PASS] treasury=terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7
  [PASS] default_fee_bps=180
  [PASS] governance (expected)=terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7
  [PASS] treasury (expected)=terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7
  [PASS] default_fee_bps (expected)=180

[2/6] Factory whitelisted CW20 code IDs...
  [PASS] whitelisted_code_ids count=2 (min 2)
  code_ids: [6036,10184]

[3/6] Fee-discount tiers (query: get_tiers)...
  [PASS] fee_discount_tiers count=11 (min 11)

[4/6] Trusted router (query: is_trusted_router)...
  [PASS] is_trusted_router(terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw)=true

[5/6] Registered hooks on first pair (query: get_hooks)...
  pair: terra1klwuxas6x7p6fjde60kq70t0hu86wvt3fvyr2vgs0nn32fnv0q4qwznwp4
  hooks: []
  [PASS] no hooks registered (pool-only launch default)

[6/6] Blacklist state for clean wallet (query: blacklist_check)...
  [PASS] wallet_blacklisted(terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v)=false
  [PASS] blocked(terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v)=false

────────────────────────────────────────────────────────────────
checks: 12   failures: 0
RESULT: PASS
```

**`smoke-pool-swap` (EMBER/CORAL):** pool `total_share=100000000000`; hybrid sim offer `1000000` → `return_amount=981991`, `commission_amount=17999`. Full output: `deployments/mainnet-soft-launch/smoke-pool-swap.txt`.

**Env/address cross-check (`qa-verify-env-addresses`):** Coolify frontend/indexer not pinned yet — defer until `frontend.env.example` / `indexer.env.example` are applied on `dex.cl8y.com` / `indexer.dex.cl8y.com`.

### Additional on-chain checks (manual)

| Check | Result |
|-------|--------|
| Wasm admin (factory/router/fee-discount) | multisig `terra1zlmv2…` |
| Factory/fee-discount `config.governance` | multisig (after [7b] handoff) |
| Factory treasury | multisig |
| Factory `pair_code_id` / `lp_token_code_id` | 11506 / 6036 |
| `get_pair_count` | 10 |
| Fee-discount `cl8y_token` | `terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3` |
| Router `factory` | soft-launch factory address |
| Whitelist | **only** `[6036, 10184]` (SL1) |
| Tiers 0–9 + 255 | present (`get_tiers` count 11) |
| Per-pair `set_discount_registry` + `set_discount_registry_all` | txs succeeded (LCD raw state probe blocked with 403 on publicnode) |

### Notes

- **Canonical stack** is the second instantiate (bootstrap governance). Do **not** use orphan first attempt: factory `terra1weddl9adjexzz82v2cyyh9x9mleneear0aeu55eyj22287pzsyls3c4qjz`, fee-discount `terra15a5s3s9wexcy6dvlrjua4quq03mm6ve7rh93ttvf4pup8rvlqyfqsc2sf2` (Unauthorized on `add_tier`).
- Mid-deploy publicnode RPC resets / sequence mismatch; completed via `./scripts/resume-mainnet-soft-launch-pairs.sh`.
- Soft launch = non-economic gemstones only; Coolify cutover still outstanding after this chain trace.
- Soft-launch faucet (#473): code **11509** / `terra1388y0ppe2c3dy4nrmnpqp7e4ggukkrnmpzfjadfeu0pu2rm9cvkslfzcen` — see [`faucet-trace.md`](./faucet-trace.md). `AddMinter` granted on EMBER…TOPAZ; primary minter remains `cl8ydeploy`. **Not** on factory CW20 whitelist (F7).
- `make qa-verify-deploy-config` defaults to LocalTerra RPC; for mainnet set `TERRA_RPC_URL` / `TERRA_LCD_URL` and provide factory/router/fee addresses via `indexer/.env` (script does not yet honor bare `FACTORY_ADDRESS=` env alone).

### Post–soft-launch: router ↔ wrap-mapper wiring (#502)

Soft launch **SL5** left wrap out of scope (`wrap_mapper` was `null` on the router). After ustr-cmm treasury migration + wrap-mapper deploy, factory governance (2-of-3 multisig) connected the mapper.

| Field | Value |
|-------|-------|
| **Date (UTC)** | 2026-08-08T00:18:31Z |
| **Height** | `29849291` |
| **Tx** | [`EAE9C840E388946E0DF4C9E0D1F9E5E3377F79911B2B39418F029F1B3B9C0754`](https://finder.terraclassic.community/mainnet/tx/EAE9C840E388946E0DF4C9E0D1F9E5E3377F79911B2B39418F029F1B3B9C0754) |
| **Msg** | `SetWrapMapper` on router |
| **Sender** | governance multisig `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7` (2-of-3; EOA `cl8y2_admin` correctly rejected as Unauthorized) |
| **Router** | `terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw` |
| **Wrap-mapper** | `terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2` |
| **Gas** | wanted `285084` / used `229015`; fee `8075005uluna` |

**Post-tx router `config` query (verified):**

```json
{"factory":"terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea","wrap_mapper":"terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2"}
```

**Auth note:** `SetWrapMapper` requires `info.sender == factory.config.governance` (not wasm admin alone, not wrap-mapper governance). Use multisig generate-only → 2 signer `tx sign --multisig` → `tx multisign` → `tx broadcast` ([governance emergency rehearsal signing flow](../../docs/runbooks/governance-emergency-rehearsal.md#signing-flow-cosmos-multisig)).

### Post-SL5 wrap contracts (#507)

Phase 3 ustr-cmm wrap stack (columbus-5). Router address unchanged from soft launch.

| Role | Address |
|------|---------|
| CMM treasury (`VITE_TREASURY_ADDRESS`) | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` |
| Wrap-mapper | `terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2` |
| cLUNC CW20 | `terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg` |
| cUSTC CW20 | `terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch` |
| Router | `terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw` |

Wrap-mapper `fee_bps`: **100** (1%). Coolify enablement: [`wrap-enablement.env.example`](./wrap-enablement.env.example); playbook [`skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md`](../../skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md).

### F6 / #584 code-id pin migrate (columbus-5, 2026-08-21)

Factory-first order. Evidence: [#584](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/584) close comment. **#581 / 8266** REPORT is **GO**; factory listed **8266** 2026-08-22 (see below). ALPHA **8654** stays off.

| Field | Value |
|-------|-------|
| **LCD** | `https://terra-classic-lcd.publicnode.com` |
| **Factory** | `terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea` |
| **Factory store** | code **11602**, tx `9ABEF0B2207646188BEFD6F702ED23ADE3795A592227B2A9263F2E6BAA82BF54` height 30039700 |
| **Pair store** | code **11601**, tx `6C44832EBE2E81B5FA0158F5203E9618332C3FD1C4ABCC32B3B3212DCE0F66F9` height 30039698 |
| **Factory migrate** | cw2 **1.8.0 → 1.9.0**, tx `794AAB9E3AEBF32E7D0050F260EC907722CEC8CA955B00CAA1608B235D0600DB` |
| **`UpdateConfig { pair_code_id: 11601 }`** | tx `F8F97C113758DB50B180BEA76E994CA8D1AB567A5969273161B3FAA7EE8E1787` height 30039822 |
| **GetPairCount** | 14 / 14 pairs on **11601** / 1.15.0 |
| **Listed assets** | **10184** (hub/CL8Y/gems) + **6036** (PEARL/QUARTZ) |

Partial run left three pairs on 11586 after publicnode RST mid `tx sign`; retry completed. Smoke `pin1=…/null` on a later LCD read was a dropped `IsCodeIdWhitelisted`, not an unlisted pin — independent re-query was 10184/10184. Upgrade script now retries those reads, skips already-migrated pairs, and `UpdateConfig`s `pair_code_id` before pair migrate.

### 8266 whitelist (columbus-5, 2026-08-22)

Template **GO**: [`cw20-codeid-audits/codeids/8266/REPORT.md`](../../cw20-codeid-audits/codeids/8266/REPORT.md) ([#581](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/581)). DEX 2-of-3 `AddWhitelistedCodeId 8266`. SpaceUSD/UST1 create+provide stays on [#558](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/558). Do **not** whitelist a LocalTerra store id. Do **not** whitelist ALPHA **8654**.

| Field | Value |
|-------|-------|
| **Factory** | `terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea` |
| **Execute** | `AddWhitelistedCodeId { code_id: 8266 }` |
| **Tx** | [`BA7C9FD05D0FC0BC14713A53325E1F42AAAECF3A11E4EC3BC91368013D8FC5D1`](https://finder.terraclassic.community/columbus-5/tx/BA7C9FD05D0FC0BC14713A53325E1F42AAAECF3A11E4EC3BC91368013D8FC5D1) |
| **Height** | **30060600** (2026-08-22T10:53:20Z) |
| **Signer** | `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7` |
| **`GetWhitelistedCodeIds`** | **`[6036, 8266, 10184]`** (11611 added 2026-08-23 — see below) |

### Community tax CW20 store + whitelist + launcher (columbus-5, 2026-08-23)

Ops [#601](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/601). Template **GO**: [`cw20-codeid-audits/codeids/11611/REPORT.md`](../../cw20-codeid-audits/codeids/11611/REPORT.md). Named T592 exception — inbound 1:1. Do **not** whitelist launcher **11612** / **11614**, AutoLP **11613**, a LocalTerra store id, or ALPHA **8654**.

| Role | `code_id` | `data_hash` | store tx | height |
|------|-----------|-------------|----------|--------|
| Token (listed) | **11611** | `9D33BF2539A9A5B2F13FD4B321CDBD0B0FD86D936D5D6BD6681955FA30210EC2` | [`C610FB95…2BF2`](https://finder.terraclassic.community/columbus-5/tx/C610FB95B4BF18F5C96B972545D8649993461FC631A219F80C927DF5172B2BF2) | 30071140 |
| Launcher (superseded) | 11612 | `A0F95FBA548F4EA6F5536CE564922EC39B9ECE691CF3F01FF8733B7A93223F19` | [`3907A435…9E8A`](https://finder.terraclassic.community/columbus-5/tx/3907A435FE6EB2EFEBB314869A031817FD64E5FB6D922A0D44E22C07E93B9E8A) | 30071141 |
| AutoLP | 11613 | `B110CCD6E543AD5F42663D8A087B58D1993D3342DD2326559DFAB8656F8F828B` | [`84A8501C…A95C`](https://finder.terraclassic.community/columbus-5/tx/84A8501C35A20A40F490479350E9270E4FC3AA1492EE6C5DB7BD6D870B44A95C) | 30071142 |
| Launcher (canonical, `CreateToken`) | **11614** | `04F57008E019172C04A25173E5F6FE6C975D67285CF953747F8F724434E44539` | [`33F6A49F…45B8`](https://finder.terraclassic.community/columbus-5/tx/33F6A49F7221A377132D0A2B534A48D5AC64A5CA1F30D20BBE8A34086D3A45B8) | 30072268 |

Uploader: `cl8ydeploy` (`terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv`). Instantiate permission **Everybody**.

| Field | Value |
|-------|--------|
| **`AddWhitelistedCodeId 11611`** | [`241FE20E…B30E`](https://finder.terraclassic.community/columbus-5/tx/241FE20E7649738DF8E34B778AE171E803C2962C1FC556242DFC1CE0A53CB30E) height **30071160** (2026-08-23T04:12:54Z) |
| **Signer** | `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7` |
| **`GetWhitelistedCodeIds`** | **`[6036, 8266, 10184, 11611]`** |
| **Launcher (canonical)** | `terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze` |
| **Launcher instantiate** | [`041E3C43…87FE`](https://finder.terraclassic.community/columbus-5/tx/041E3C4379E88CE073B2EEED0125BEC58BCCAC31711AC15500352581763287FE) height **30072275** |
| **Launcher wasm admin** | `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7` (DEX 2-of-3) |
| **Token `cmm_governance` / CMM** | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` |
| **Unused 11612 instance** | `terra1af9xm63mev4hnf4z0nmmcsnd9f4lpac2vs205rmaeg3kdqlqudhq894lyz` (CMM treasury admin; no `CreateToken`) |

Coolify / indexer (dApp [#593](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593), indexer [#594](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/594)): `VITE_COMMUNITY_TAX_CODE_ID=11619`, `VITE_COMMUNITY_TOKEN_LAUNCHER` / `COMMUNITY_TOKEN_LAUNCHER` = canonical launcher above, `CMM_GOVERNANCE_ADDR` = CMM, `COMMUNITY_TAX_OPTION2_CODE_IDS=11619`. Unset keeps Create Token / catalog unconfigured.

### Community tax rotate (#611 / #612 / #616, columbus-5, 2026-08-24)

Script: [`scripts/upgrade-611-community-tax.sh`](../../scripts/upgrade-611-community-tax.sh). Tip includes !409 option-2 classify. Uploader `cl8ydeploy`. Instantiate permission **Everybody**.

| Role | `code_id` | `data_hash` | store tx | height |
|------|-----------|-------------|----------|--------|
| Token (stored + listed 2026-08-24) | **11619** | `63CB21D1806C5DA65818AEABCDB4727C71709862B7E4C7042F99CFB34CBAFA20` | [`42A76F85…CFDE`](https://finder.terraclassic.community/columbus-5/tx/42A76F85B687C3E8DF548193E11CDBAC92A4D6934C877F76CF85EE97806CCFDE) | 30085543 |
| Launcher (canonical instance now this wasm) | **11620** | `7AD7DBA2200BAACF61C8B4D1088F3B55F22B8CDCB3BBB10EF5DEDFC7A78E87BF` | [`672F9464…AEC2`](https://finder.terraclassic.community/columbus-5/tx/672F946410FD5FC89953DD9F033FCD17171AAAFA14647A5291341F7335A5AEC2) | 30085545 |
| AutoLP (stored, **not** listed) | **11621** | `DAD413A3207291BB697BEE703CA590C27825BA64BE3D174361D4861F108B76F1` | [`92F90C51…C8A6`](https://finder.terraclassic.community/columbus-5/tx/92F90C510B604D8413AAA366D37C3E3F8F1B22230A3AD7DB8B39F22EBAA5C8A6) | 30085547 |

| Field | Value |
|-------|--------|
| **Launcher migrate** `terra126pr5…` **11614 → 11620** | [`97C0FCA9…EE8C`](https://finder.terraclassic.community/columbus-5/tx/97C0FCA93DFADD4BE4250935C7EFAF1CAB0A20C6FB64B2D8B774A4A8BF63EE8C) height **30085550** |
| **Signer** | DEX 2-of-3 `terra1zlmv2…hep7` |
| **`GetConfig`** | `token_code_id` **11611**, `autolp_code_id` **11613** (no `UpdateConfig` on launcher) |
| **`GetWhitelistedCodeIds`** | **`[6036, 8266, 10184, 11611, 11619]`** — `AddWhitelistedCodeId 11619` [`B659D914…40CF`](https://finder.terraclassic.community/columbus-5/tx/B659D914CEAA045F05B5371F90669B79E34E7AE414537C6B4A8A58286BC640CF) height **30085794** (DEX 2-of-3) |
| **11611 / 11613 / 11619 / 11621 instances** | **0** — no CMM migrate |

Do **not** whitelist **11620** / **11621** / **11622** / unused **11612** / ALPHA **8654**. Keep **11611** listed (F6) until every pair pin is refreshed.

### Launcher UpdateConfig (#611, columbus-5, 2026-08-24)

| Field | Value |
|-------|--------|
| Launcher store **11622** | pin `8E56AE0FDA1336EB3C39EC67E6C49D7A002E8E70E30D0766EEE69BA3D2098E90` · store [`FA30A5B1…2355`](https://finder.terraclassic.community/columbus-5/tx/FA30A5B1CD87E4D984123EE17BA05D7CFE6F852D1A93EFAB293AB7CBE0B12355) |
| Migrate `terra126pr5…` **11620 → 11622** | [`F2166AB0…AAB2`](https://finder.terraclassic.community/columbus-5/tx/F2166AB0C09B4E7989AB10DC8DCC4D5855B4E3F91C7E4F8C6D5B8F780947AAB2) height **30086055** (DEX 2-of-3) |
| `UpdateConfig` | [`DAC86F27…6ED3`](https://finder.terraclassic.community/columbus-5/tx/DAC86F27B4E95FC83461B733453A9EF1028BC8421F2FE3AE022B0A14DADF6ED3) height **30086058** |
| **`GetConfig`** | `token_code_id` **11619**, `autolp_code_id` **11621** |
| Instances | still **0** — no CMM migrate / no Refresh |

Coolify leftover: flip `VITE_COMMUNITY_TAX_CODE_ID` / `COMMUNITY_TAX_CODE_ID` to **11619** (catalog is single-id) and set `COMMUNITY_TAX_OPTION2_CODE_IDS=11619`. Do not factory-list **11622**.

### Community tax adopt pin (#628, columbus-5, 2026-08-24)

Script: [`scripts/upgrade-611-community-tax.sh`](../../scripts/upgrade-611-community-tax.sh) with `UPGRADE611_SKIP_STORE=1` / `UPGRADE611_TOKEN_CODE_ID=11626`. Uploader `cl8ydeploy`. Instantiate permission **Everybody**.

| Field | Value |
|-------|--------|
| Token (stored + listed 2026-08-24) | **11626** · pin `A7244C93D8490CFD1063DC7B45C3F09E38E947008A0C6069A93E6191C2D9DA1C` · store [`95D5D44C…7E01`](https://finder.terraclassic.community/columbus-5/tx/95D5D44C07AB2284DD377684265C7E2BE573D05D52612FF2B8388B0896F97E01) height **30091582** |
| **`AddWhitelistedCodeId 11626`** | [`03F74C9B…DC91`](https://finder.terraclassic.community/columbus-5/tx/03F74C9BEFF145732E1A358E06F785DBC499D8529096BF599191E5967896DC91) height **30091644** (DEX 2-of-3) |
| **`GetWhitelistedCodeIds`** | **`[6036, 8266, 10184, 11626]`** after `RemoveWhitelistedCodeId 11619` [`9591AC1E…ABFE`](https://finder.terraclassic.community/columbus-5/tx/9591AC1E8E39734081C0EA304C255253C47009368C07C6EB4FE3325822C1ABFE) height **30091710** |
| **`GetConfig`** | `token_code_id` **11626** / `autolp_code_id` **11621** — `UpdateConfig` [`ED5E5725…E80A`](https://finder.terraclassic.community/columbus-5/tx/ED5E57255689A497BF391374DEFE988B1D13072E1693243F8081B65A78DFE80A) height **30091680** |
| **11611 / 11619 / 11626 instances** | **0** — no CMM migrate / no Refresh |
| **Pair pins** | 16/16 factory pairs are 6036 / 8266 / 10184 only — no 11619 |

Do **not** whitelist **11621** / **11622** / unused **11612** / unused **11619** / ALPHA **8654** / code **3**. Coolify leftover: frontend rebuild must bake `VITE_COMMUNITY_TAX_CODE_ID=11626` (indexer catalog already `code_id=11626`). REPORT: [`cw20-codeid-audits/codeids/11626/REPORT.md`](../../cw20-codeid-audits/codeids/11626/REPORT.md).

### Community tax #633 pin (#635, columbus-5, 2026-08-25)

Factory migrate + token list + launcher AutoLP pin. Uploader `cl8ydeploy`. Instantiate permission **Everybody**.

| Field | Value |
|-------|--------|
| Factory (live) | **11629** · pin `64ABBCCB…22982C` · unused prior store **11628** same hash |
| Token (stored + listed) | **11630** · pin `E60A6E31BB0261505170208BBF2B0E17CDB4C6258D701E2E6D22E362CA872B96` · store [`11763CB7…9E48`](https://finder.terraclassic.community/columbus-5/tx/11763CB747BEE51231871F580A36862A47330D4C9EBF220FC779A0E16DC99E48) height **30100763** |
| Duplicate token store | **11631** · same pin as **11630** — do not list · store [`627DD94A…CC6A`](https://finder.terraclassic.community/columbus-5/tx/627DD94A26885736C189ABE5EFCA2E594C128E565CA90CEF484F827AE810CC6A) |
| Launcher wasm | **11632** · migrate [`68B5A234…5D01`](https://finder.terraclassic.community/columbus-5/tx/68B5A234D5C14A2CFEC4A4264E04E2E8915BAA2BD51EAC7E582E7A80AFE65D01) height **30101035** |
| AutoLP store | **11633** · pin `A5E56F61FCCA59232E37DCA283F2E300731B999E60C6813D274FD8A5CEE95934` · store [`6CBD04B6…ED00`](https://finder.terraclassic.community/columbus-5/tx/6CBD04B6448E162D68CF4AE95B421008435EB3FD316C60D29A9E3E5BC52BED00) |
| **`GetWhitelistedCodeIds`** | **`[6036, 8266, 10184, 11630]`** (11626 removed; 0 instances) |
| **`GetConfig`** | `token_code_id` **11630** / `autolp_code_id` **11633** — AutoLP pin [`E2259FB0…5B03`](https://finder.terraclassic.community/columbus-5/tx/E2259FB0001648E92CB5144ACBCA3347ADA3AF05754E9478FC3071D77DFC5B03) height **30101052** |
| **11611 / 11619 / 11626 / 11630 / 11633 instances** | **0** — no CMM migrate / no Refresh |

Do **not** whitelist **11631** / **11632** / **11633** / **11628** / **11629** / **11621** / **11622** / unused **11612** / ALPHA **8654**. Coolify/indexer bake **11630**. REPORT: [`cw20-codeid-audits/codeids/11630/REPORT.md`](../../cw20-codeid-audits/codeids/11630/REPORT.md).

### DEX 2-of-3 accept queue (#525 / #526, columbus-5, 2026-08-25)

App-governance accepts from `cl8y2_admin` pending transfers. Signer `multisig_2of3` (`multisig1` + `multisig2`) via [`scripts/multisig-2of3-host-tx.sh`](../../scripts/multisig-2of3-host-tx.sh).

| Field | Treasury `AcceptGovernanceTransfer` | Wrap-mapper `accept_governance_transfer` |
|-------|-------------------------------------|------------------------------------------|
| Contract | `terra16j5u6…3lr2` | `terra1xuuuh…nmd2` code **11574** |
| Tx | [`E819E594…B9A1`](https://finder.terraclassic.community/columbus-5/tx/E819E59471BCD82A230AF5771F062B2D7F984F6B12EAEB2FD146F83248F4B9A1) | [`02F139F1…00A2`](https://finder.terraclassic.community/columbus-5/tx/02F139F16BEA7322EC87C14FC8EB4EEEFB9E66B6CB2DED2D95AEC291FE0F00A2) |
| Height | **30101431** | **30101435** |
| `config.governance` after | DEX 2-of-3 `terra1zlmv2…hep7` | DEX 2-of-3 `terra1zlmv2…hep7` |
| Pending after | `proposals: []` | `new_governance: null` |
| Wasm admin (unchanged) | `cl8y2_admin` `terra1xsecn4…` | `cl8y2_admin` `terra1xsecn4…` |

Window / oracle / swap / bridge accepts were **not** in this batch (still EOA). Registry: [`../mainnet-ust1-wrap/REGISTRY.md`](../mainnet-ust1-wrap/REGISTRY.md).

