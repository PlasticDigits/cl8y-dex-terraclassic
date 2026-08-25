# 11630 (`cl8y-community-tax-token` #633 manager-role skip)

Named T592 exception for the #589 harness ([#635](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/635) leftover of [#633](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/633)). Same-crate bump of **11626** plus `config.manager` tax skip (no paid ExemptionDirectory row). **GO** 2026-08-25; factory-listed 2026-08-25.

- Pin: [`wasm.sha256`](wasm.sha256) = LCD `data_hash` `E60A6E31…CA872B96`
- Report: [`REPORT.md`](REPORT.md)
- Adopt pin remains [`../11626/REPORT.md`](../11626/REPORT.md)
- Listed rotate pin remains [`../11619/REPORT.md`](../11619/REPORT.md)
- Honest pin remains [`../11611/REPORT.md`](../11611/REPORT.md)
- Canonical launcher: `terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze` (code **11632**; `token_code_id` **11630** / `autolp_code_id` **11633**)

```bash
./cw20-codeid-audits/scripts/fetch-lcd-wasm.sh 11630
./cw20-codeid-audits/scripts/decompile-wasm.sh 11630
CODE_ID=11630 make verify-issue-589
CODE_ID=11630 LAYER_B_LT=1 make verify-issue-589
```
