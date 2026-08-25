# 3 — columbus-5 original Everybody CW20 (`cw20-legacy`)

Investigation only ([#627](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/627)). **NO-GO** for (A) migrate-adopt and (B) factory list.

- Pin: [`wasm.sha256`](wasm.sha256) = LCD `data_hash` `F9B4AB22…B44891`
- Report: [`REPORT.md`](REPORT.md)
- Census snapshot: [`census.json`](census.json)
- Playbook: [`../../../skills/AGENTS_CW20_CODE_ID_3.md`](../../../skills/AGENTS_CW20_CODE_ID_3.md)

```bash
./cw20-codeid-audits/scripts/fetch-lcd-wasm.sh 3
./cw20-codeid-audits/scripts/fingerprint-wasm.sh 3
./cw20-codeid-audits/scripts/decompile-wasm.sh 3
make verify-issue-627
# Do **not** treat CODE_ID=3 LAYER_B_LT=1 as a listing path (interface_version_7).
```

Do **not** append `3` to `VITE_COMMUNITY_MIGRATE_CODE_IDS` or call `AddWhitelistedCodeId 3`.
