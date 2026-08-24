# 11626 (`cl8y-community-tax-token` adopt pin)

Named T592 exception for the #589 harness ([#628](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/628)). Post-#626 foreign `adopt.rs` pin. **GO** 2026-08-24; factory-listed 2026-08-24. Keep **11619** listed until Refresh.

- Pin: [`wasm.sha256`](wasm.sha256) = LCD `data_hash` `A7244C93…D9DA1C`
- Report: [`REPORT.md`](REPORT.md)
- Listed rotate pin remains [`../11619/REPORT.md`](../11619/REPORT.md)
- Honest pin remains [`../11611/REPORT.md`](../11611/REPORT.md)
- Canonical launcher: `terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze` (code **11622**; `token_code_id` **11626**)

```bash
./cw20-codeid-audits/scripts/fetch-lcd-wasm.sh 11626
./cw20-codeid-audits/scripts/decompile-wasm.sh 11626
CODE_ID=11626 make verify-issue-589
CODE_ID=11626 LAYER_B_LT=1 make verify-issue-589
```
