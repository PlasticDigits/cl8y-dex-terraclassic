Bridge-Debt issuer/receipt 1.1.0 uses the observed multisig owner/admin and explicit reviewed USDT10184/receipt pins. It stores only the two checksum-pinned artifacts, migrates the receipt and issuer, installs finite10,000 USDT caps, and keeps new issuance paused. It creates no LP or additional debt.

Run these commands in order from the repository root. Only `execute` and `probes` unlock the encrypted file keyring. The probes repay one raw unit (10^-18 USDT) using `cl8y2_admin`, then prove recovery by returning one plain-transferred raw unit to that same wallet.

```bash
python3 scripts/ops/2026-10-06/bridge-debt.py plan
python3 scripts/ops/2026-10-06/bridge-debt.py execute
python3 scripts/ops/2026-10-06/bridge-debt.py verify
python3 scripts/ops/2026-10-06/bridge-debt.py probes
```

The durable journal lives in `~/.local/state/cl8y-dex/release-20261006-bridge-debt`. Resume with the same journal. Each signed transaction is saved and hashed before its single broadcast. Unknown inclusion stops; `--rebroadcast-pending` deliberately replays identical saved bytes after receipt reconciliation. No fresh signing retries occur. Do not delete or edit the manifest/journal to bypass failure.

The aggregate fee limit is2,000,000,000 uluna (2,000 LUNC), with25M gas per transaction. Already matching limits/defaults/migrations are skipped. Reruns reconcile receipts and reassert debt supply/outstanding instead of repeating repayment/recovery. The issuer owner and wasm admins remain the observed2-of-3 multisig; these changes do not establish collateral backing or remote bridge custody.
