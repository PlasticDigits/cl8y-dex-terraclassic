# 11611 (`cl8y-community-tax-token`)

Named T592 exception for the #589 harness ([#592](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592) / [#601](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/601)).

- Pin: [`wasm.sha256`](wasm.sha256) = LCD `data_hash` `9D33BF25…210EC2`
- Report: [`REPORT.md`](REPORT.md)
- Columbus-5 listed 2026-08-23. Canonical launcher: `terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze` (code **11614**)

```bash
./cw20-codeid-audits/scripts/fetch-lcd-wasm.sh 11611
./cw20-codeid-audits/scripts/decompile-wasm.sh 11611
CODE_ID=11611 make verify-issue-589
CODE_ID=11611 LAYER_B_LT=1 make verify-issue-589
```
