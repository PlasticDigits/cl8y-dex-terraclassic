# 8266 (Terraport token V2)

First candidate for the #589 harness ([#581](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/581) SpaceUSD/UST1).

- Pin: [`wasm.sha256`](wasm.sha256) = LCD `data_hash` `953AD60C…`
- Report: [`REPORT.md`](REPORT.md)
- Prior one-off notes (superseded as intake path): [`../../../audits/CW20-8266-581.md`](../../../audits/CW20-8266-581.md)

```bash
./cw20-codeid-audits/scripts/fetch-lcd-wasm.sh 8266
./cw20-codeid-audits/scripts/decompile-wasm.sh 8266
CODE_ID=8266 make verify-issue-589
```
