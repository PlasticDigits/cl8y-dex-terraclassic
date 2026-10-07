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
issue notes. The 2026-10-07 close separates contract verification from the
funded smoke and Bridge-Debt keyring steps. See
[docs/qa/issue-1384/README.md](../../../docs/qa/issue-1384/README.md) and
[REL1421-1–REL1421-10](../../../skills/AGENTS_RELEASE_OPS_1421.md).

Agent work already completed includes production database isolation/migrations,
retained history repair, indexer/frontend rollout, owned funded chain controls,
the trusted policy deployment, dedicated daily checks and reproducible native
builds. #1385 is closed. The liquidity display retains its last qualified value;
current API quality and percentage comparability remain independently qualified.

The [route instructions](ROUTE-README.md) and
[Bridge-Debt instructions](BRIDGE-DEBT-README.md) document the journals and bounded
spending. The single consolidated inbox card contains the original DEX issue URLs.

The interrupted October 7 rollout has an audited artifact correction: the token
and pair are rebuilt from unchanged locked source with the pinned Rust 1.81
compiler's size profile. Only unsigned, unstored artifacts may change. Original
manifest, inventory, pause states and included transactions remain archived and
linked to the amendment. The preflight now checks the production 100-local limit
before any signing. Application code stays pinned while operational fixes may
follow it only with identical application and contract source inputs.

The keyring passphrase passes to custody children through a bounded anonymous
pipe, not an environment value. SDK commands receive it only on stdin. Route,
smoke and containment fees share the route cap, with funds reserved for both
containment and restoration before signing.

The repair also freezes a current economic checkpoint while every affected pair
is paused and every token/pair still has its original implementation. This
checkpoint is bound to the immutable amendment receipt; the earlier inventory
and original pause flags are retained. Historical providers did not supply
height-attested snapshots, so the checkpoint establishes the state before the
remaining migrations and does not attribute prior swaps or LP changes. Any
subsequent pool or supply drift fails verification.
