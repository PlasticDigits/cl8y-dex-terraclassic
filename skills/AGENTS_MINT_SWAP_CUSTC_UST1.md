# Agent playbook: hourly $200 cUSTC mint → best-solver UST1

Use when running or changing the **$200-base cUSTC** mint → indexer **best solver** swap to **UST1** until CMM **vFDUSD CW20 × Venus × FDUSD oracle** is **$500**. Amounts default to ±20%, and waits between completed ticks default to 3600s ±900s.

## Canonical references

| Doc / script | Purpose |
|--------------|---------|
| [`docs/runbooks/mint-swap-custc-ust1.md`](../docs/runbooks/mint-swap-custc-ust1.md) | Operator runbook |
| [`scripts/mint-swap-custc-ust1.sh`](../scripts/mint-swap-custc-ust1.sh) | Host terrad: 2-of-3 mint → `/route/solve/best` → CMM |
| [`scripts/lib/custc-ust1-buyback-defaults.sh`](../scripts/lib/custc-ust1-buyback-defaults.sh) | Columbus-5 router / token / treasury anchors |
| [`scripts/lib/custc-ust1-buyback-math.py`](../scripts/lib/custc-ust1-buyback-math.py) | Mint-from-USD, vFDUSD USD (`self-test`) |
| [`scripts/lib/ust1-clunc-buyback-math.py`](../scripts/lib/ust1-clunc-buyback-math.py) | Shared send-hook / min-receive |

## Rules of thumb

1. **Unlock once** — prompt or `TERRAD_HOST_KEYRING_PASS`. Never commit it. File-keyring keys share one passphrase.
2. **Mint via DEX 2-of-3 extra minter** (`terra1zlmv2…`) to the admin hot wallet (`cl8ydeploy` by default). Size is a **$200 base ±20% at USTC/USD**, not 200 tokens. Draw fresh amounts using OS randomness; validate the quote and router hook before minting. Extra-mint is unbacked wrap (warn when supply > CMM `uusd`).
3. **Execute indexer `GET /api/v1/route/solve/best`** ops as-is (hybrid hops included). Do **not** attach greedy (`G4`). `minimum_receive` uses **pre-tax** `estimated_amount_out`, not `estimated_amount_out_net`, with **0.25%** slippage tolerance by default (99.75% of quote). Router hops with `book_input > 0` need per-hop `min_return` (#334) — indexer omits it; this script LCD-sims each hop and attaches the same tolerance (fallback `1`).
4. **Never unwrap. Never window-redeem.** Unwrap would move CMM `uusd`. Window redeem would drain CMM vFDUSD (the stop). Default dest **CW20-transfers** the tick’s UST1 to CMM. The tick does **not** raise vFDUSD; third-party window deposits are expected to.
5. **Stop on CMM vFDUSD × Venus × FDUSD/USD only.** Do not add CMM’s UST1. Do not stop on swapped USD. Do not assume 1 vFDUSD = $1. Live default **loops with fresh 45–75 minute waits after each tick** until that USD ≥ `$CUSTC_UST1_TARGET_USD` (500). Never align waits to wall-clock hours. Dry-run is one tick.
6. Prefer `DRY_RUN=1` before a live broadcast. Live needs `CUSTC_UST1_YES=1` when stdin is not a TTY. A tick `die` in loop mode sleeps and retries rather than exiting.
7. **Mint preparation retries** — generate-only, both signer commands, and multisign use `terrad_host_exec_retry` (six attempts; 4/6/8/10/12-second waits). Permanent errors stop immediately, and failed stdout must never reach the transaction/signature files. Do not wrap the whole mint or tick in a short-retry loop. Multisig broadcast is a separate single attempt.
8. **RPC** — this buyback defaults to `https://rpc.terra-classic.hexxagon.io:443`; set it before sourcing shared host helpers, respect `TERRAD_HOST_NODE`, and print the node in the header. Restart a running loop to load changes.

## Quick commands

```bash
python3 scripts/lib/custc-ust1-buyback-math.py self-test
DRY_RUN=1 ./scripts/mint-swap-custc-ust1.sh
CUSTC_UST1_YES=1 ./scripts/mint-swap-custc-ust1.sh
CUSTC_UST1_LOOP=0 CUSTC_UST1_YES=1 ./scripts/mint-swap-custc-ust1.sh
```

## Related

- [`AGENTS_MINT_SWAP_BURN_UST1_CLUNC.md`](./AGENTS_MINT_SWAP_BURN_UST1_CLUNC.md) — sister 50 UST1 → cLUNC burn until CMM bank `uluna` is $2500
- [`AGENTS_INDEXER_VENUS_VFDUSD.md`](./AGENTS_INDEXER_VENUS_VFDUSD.md) — Venus redeem is not CEX FDUSD/USD
- [`AGENTS_HYBRID_QUOTING.md`](./AGENTS_HYBRID_QUOTING.md) — quote = execute via `/route/solve`
- [`AGENTS_KEY_CUSTODY.md`](./AGENTS_KEY_CUSTODY.md) — 2-of-3 roster
