# Soft-launch faucet deploy trace (GitLab #473)

- **When:** 2026-07-12T06:28Z
- **Chain:** columbus-5
- **Deployer / faucet admin / primary CW20 minter:** `terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv`
- **FAUCET_CODE_ID:** `11509`
- **FAUCET_ADDRESS:** `terra1388y0ppe2c3dy4nrmnpqp7e4ggukkrnmpzfjadfeu0pu2rm9cvkslfzcen`
- **drip_amount:** `100000000` (100 human units @ 6 decimals)
- **cooldown_seconds:** `300` (global per wallet)
- **Allowlist:** EMBER, CORAL, JADE, ONYX, RUBY, TOPAZ (not QUARTZ/PEARL)
- **F6:** `cl8ydeploy` remains primary CW20 minter; no governance minter handoff
- **F7:** Faucet code id is **not** on factory CW20 whitelist

## AddMinter txs

- `EMBER:40D7A4A03139F2E73C93E6921DE893587F613EFA21342DE85967FDE1C10A48C3`
- `CORAL:703291044D82F91212284DE0FD462809311C8B967AC8AEEEBDC59268B14CE5E8`
- `JADE:254EFA87B03F6D7B63C32450330AD3CD8AA29F9CF0BD442CFB97D00A34C432E7`
- `ONYX:93BFD724371DFEA9382C50DB7D5E8D5AEA62B6DAC6F7D6A7D8915C09BFB663C1`
- `RUBY:0AA4B01A37D83A0B449747F5555C2B96409640EA0B40ADA3DAA0C4B886C96677`
- `TOPAZ:1C829FBD585CE62D957AF79552EEF6579CC4A19B34A71679DCBA386F4CF42B71`

## Verification

```bash
# Faucet config
terrad query wasm contract-state smart terra1388y0ppe2c3dy4nrmnpqp7e4ggukkrnmpzfjadfeu0pu2rm9cvkslfzcen '{"config":{}}' --node https://terra-classic-rpc.publicnode.com:443 --output json
# Sample Minters (EMBER)
terrad query wasm contract-state smart $TOKEN_EMBER_ADDRESS '{"minters":{}}' --node https://terra-classic-rpc.publicnode.com:443 --output json
```
