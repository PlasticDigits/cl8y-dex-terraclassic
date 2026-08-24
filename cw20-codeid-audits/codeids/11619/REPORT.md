# REPORT: CW20 code ID 11619 (`cl8y-community-tax-token` rotate)

**Date:** 2026-08-24  
**Operator:** ops  
**LCD:** columbus-5 `https://terra-classic-lcd.publicnode.com`  
**Procedure:** [`PROCEDURE.md`](../../PROCEDURE.md) ([#589](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/589))  
**Issue:** [#611](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/611) / [#612](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/612) / [#616](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/616)

Stored from current `main` (option-2 classify + #604–#609). **Not** factory-listed. Listed pin stays [`../11611/REPORT.md`](../11611/REPORT.md).

## Verdict

**GO / NO-GO** for factory `AddWhitelistedCodeId`:

- [ ] GO
- [x] **NO-GO** — intake not run (fetch / decomp / Layer A-lcd + B-lt / catalogue)

One-line reason: LCD store landed; do not list until this report is **GO**.

Approving an ID admits **every** instantiate of that wasm (including rogue `--admin`). Catalog must still filter `ContractInfo.admin == CMM` and `GetLauncherOrigin`.

## Identity

| Field | Value |
|-------|--------|
| `code_id` | **11619** |
| `data_hash` (LCD) | `63CB21D1806C5DA65818AEABCDB4727C71709862B7E4C7042F99CFB34CBAFA20` |
| Creator / uploader | `terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv` (`cl8ydeploy`) |
| Instantiate permission | **Everybody** |
| Store tx | [`42A76F85…CFDE`](https://finder.terraclassic.community/columbus-5/tx/42A76F85B687C3E8DF548193E11CDBAC92A4D6934C877F76CF85EE97806CCFDE) height **30085543** |
| Instances | **0** (2026-08-24) |

Sister stores (not listable): launcher **11620** `7AD7DBA2…` (canonical instance `terra126pr5…` migrated 11614→11620, [`97C0FCA9…EE8C`](https://finder.terraclassic.community/columbus-5/tx/97C0FCA93DFADD4BE4250935C7EFAF1CAB0A20C6FB64B2D8B774A4A8BF63EE8C)); AutoLP **11621** `DAD413A3…`. Do not whitelist **11620** / **11621**.

## Intake (not run)

```bash
./cw20-codeid-audits/scripts/fetch-lcd-wasm.sh 11619
./cw20-codeid-audits/scripts/fingerprint-wasm.sh 11619
./cw20-codeid-audits/scripts/decompile-wasm.sh 11619
CODE_ID=11619 LAYER_B_LT=1 make verify-issue-589
```

Then fill every catalogue row and flip this verdict to **GO** before `UPGRADE611_589_GO=1 ./scripts/upgrade-611-community-tax.sh`.
