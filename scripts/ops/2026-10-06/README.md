The encrypted terrad keyring is the remaining custody step. Run the prepared
release from this checkout:

```bash
bash scripts/ops/2026-10-06/run-custody.sh
```

Enter the keyring passphrase once. The wrapper refreshes OpenBao certificate
authentication, verifies the prepared application revision and artifacts, then
performs the coordinated existing-address DEX migrations, applies actual new
code identities to the apps, restores the rollout pauses, executes the bounded
native-wrap/two-hop accounting smoke, and migrates/probes Bridge-Debt. Matching
settings and completed transactions are reused. A failed or uncertain transaction
retains its original signed bytes and journal for reconciliation.

The process uses `cl8ydeploy`, `multisig_2of3` with `multisig1`/`multisig2`, and
`cl8y2_admin`. Passphrase input stays in process memory. The script records fj
issue notes; #1384 closes only after actual migration, app, provider and execution
receipts pass. Broader release scope remains recorded on #1421.

Agent work already completed includes production database isolation/migrations,
retained history repair, indexer/frontend rollout, owned funded chain controls,
the trusted policy deployment, dedicated daily checks and reproducible native
builds. #1385 is closed. The liquidity display retains its last qualified value;
current API quality and percentage comparability remain independently qualified.

The [route instructions](ROUTE-README.md) and
[Bridge-Debt instructions](BRIDGE-DEBT-README.md) document the journals and bounded
spending. The single consolidated inbox card contains the original DEX issue URLs.
