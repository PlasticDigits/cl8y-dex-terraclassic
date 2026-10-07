The route scripts bind the integrated router 1.5.0, token 1.4.0, factory 1.12.0,
pair 1.21.0 and AutoLP 1.1.0 artifacts to their exact reviewed SHA-256 values.
The live launcher 1.0.0 and CMM 0.2.2 remain compatible. All broadcasts use
`cl8ydeploy` for stores and `multisig_2of3` with `multisig1`/`multisig2` for the
inventoried governance actions. ALPHA and KENA migrate through their actual CMM
administrator. The engine checks public keys in their real 1/3/2 order.

Run these commands in an interactive terminal in this checkout:

```bash
bash scripts/ops/2026-10-06/route-01-plan.sh
bash scripts/ops/2026-10-06/route-02-execute.sh
```

Enter the keyring passphrase at the terminal prompt. It is held in memory only.
Execution contains all 22 originally unpaused markets, migrates existing addresses,
refreshes asset code pins and rotates both creation templates. It leaves the
markets paused. The durable journal is
`~/.local/state/cl8y-dex/release-20261006-route`; an interrupted run resumes the
same journal and reconciles pending hashes before another submission. Do not
delete or edit the manifest/journal to work around a failed transaction.

Apply that directory's generated `deployment.env` to the indexer/frontend, restart
the indexer to invalidate route caches, and read back the live funded quotes.
The generated values use actual stored code IDs. For the ALPHA/UST1 one-hop
forward/reverse scope, save read-only provider evidence:

```bash
bash scripts/ops/2026-10-06/route-03-verify.sh
python3 scripts/ops/2026-10-06/route-provider-readback.py \
  > "$HOME/.local/state/cl8y-dex/release-20261006-route/provider-readback.json"
```

The script uses an unfunded public synthetic query identity and checks reverse
offers against fresh forward sufficiency. It retains raw failures. The endpoint's
actual smart-query limit remains explicitly unknown unless supplied through
`--provider-smart-query-limit`; a successful quote does not measure its gas.
Populated greedy routes and broader provider qualification retain separate scope.
Then restore the rollout pauses:

```bash
bash scripts/ops/2026-10-06/route-03-verify.sh
bash scripts/ops/2026-10-06/route-04-finalize.sh
```

Finalization restores only pauses introduced by this journal. Follow it with the
bounded synthetic live route accounting smoke:

```bash
python3 scripts/ops/2026-10-06/route-live-smoke.py
```

The smoke uses 1,000,000 raw uluna as the initial route offer, wraps it and routes
cLUNC → UST1 → ALPHA in one transaction. Its fresh synthetic account starts with
zero route-token balances. A separate 150,000,000-uluna gas reserve is funded by
`cl8ydeploy`; cumulative declared transaction fees are capped at 160,000,000 uluna, including
the separate donor-funding transaction.
The private synthetic keyring remains in the receipt directory for reconciliation
and residual-gas recovery. No production key is exported. The combined release
harness can reuse its already-unlocked donor adapter through `run_smoke`.
A failed
smoke requires containment and reconciliation; migration readbacks alone do not
establish swap execution acceptance.

The October 6 read-only AutoLP inventory at height 30716647 found no deployed
sisters for codes 11613/11621/11633 and `autolp:null` on both inventoried tokens.
No policy refresh exists to execute in that observed fleet. Rotating the AutoLP
creation template does not claim migration of existing sisters or approve reserves.
Issues: [#1384](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1384),
[#1386](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1386),
[#1421](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1421).
