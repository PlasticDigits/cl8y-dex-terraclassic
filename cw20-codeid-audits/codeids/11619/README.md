# 11619 (`cl8y-community-tax-token` rotate)

Named T592 exception for the #589 harness ([#611](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/611)). Post-#604–#609 + option-2 pin. **GO** 2026-08-24; factory-listed 2026-08-24.

- Pin: [`wasm.sha256`](wasm.sha256) = LCD `data_hash` `63CB21D1…BAFA20`
- Report: [`REPORT.md`](REPORT.md)
- Listed pin remains [`../11611/REPORT.md`](../11611/REPORT.md)
- Canonical launcher: `terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze` (code **11622**)

```bash
./cw20-codeid-audits/scripts/fetch-lcd-wasm.sh 11619
./cw20-codeid-audits/scripts/decompile-wasm.sh 11619
CODE_ID=11619 make verify-issue-589
CODE_ID=11619 LAYER_B_LT=1 make verify-issue-589
```
