# Columbus-5 route rollout #1384

Issue: [#1384](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1384).
Protocol rules stay [RT1381-1–RT1381-10](../../../skills/AGENTS_ROUTE_LOCAL_TAX.md).
Rollout rules stay [RTROLL1381-1–RTROLL1381-10](../../../skills/AGENTS_ROUTE_TAX_ROLLOUT.md).
Release coordination: [REL1421-1–REL1421-10](../../../skills/AGENTS_RELEASE_OPS_1421.md)
and [#1421](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1421).
Runbook: [route-local-tax-1381.md](../../runbooks/route-local-tax-1381.md).
Pair fleet scope remains [#1386](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1386).

Checked 2026-10-07 from `dfb60143c432b17a92fcf9dde849257ebece4e1b`.
The read-only probe was `bash scripts/ops/2026-10-06/route-03-verify.sh`
against `~/.local/state/cl8y-dex/release-20261006-route`.

## Agent checklist and skip policy

Do the read-only checks first. An unavailable keyring, multisig, or device
check is **SKIP** with the reason and the existing custody command. That skip
is not a funded-execution PASS. An executed assertion failure stays **FAIL**.

- [x] Reviewed identities: factory 11711, router 11712, token 11713, AutoLP 11714,
  pair 11715, launcher 11632, CMM 11667. Journal identity
  `09495ab1239f8b3b0388bbdcb329a5a9430fcf2e12690f057cc0aba620af28ab`.
- [x] Journal `complete`: 100 included transactions, receipt code 0, fee sum
  2,174,328,427 uluna. Kinds: 22 pause, 5 store, 26 migrate, 1 whitelist,
  1 launcher template, 1 factory template, 22 asset-code refresh, 22 restore.
- [x] Live `route-03-verify.sh`: `contract_verification` PASS and
  `production_execution` UNVERIFIED. That second field is the verifier's own
  boundary: it checks code, hashes, funding capability, tax previews, whitelist,
  pins, and restored pauses. It does not broadcast a swap.
- [x] Serving indexer `d3030020d174458222a40abbf680b3379d729d2f`, chain
  `columbus-5`. `GET /api/v1/community-tokens` reports code id 11713 and two
  tokens. ALPHA→UST1 `POST /api/v1/route/solve` for 1,000,000 raw returned
  `tax_kind=route_local_v1`, `router_hops_tax=true`, and equal gross/net estimate 10.
- [x] Publicnode one-hop forward/reverse quotes at 2026-10-07T10:48:10Z: both
  checks PASS. The provider smart-query limit is still unknown, so this is not
  a 3M envelope.
- [ ] Funded native-wrap / two-hop / buy-tax execution: **SKIP**. No
  `live-smoke/` receipt exists. The encrypted terrad keyring cannot be unlocked
  unattended. This does not establish quote/net/sink/refund/gas reconciliation
  on a broadcast transaction.

## Optional custody receipt

The completed migrations are reused. The passphrase is entered once in the
terminal. This command is the leftover receipt, not a blocker after the skip
above is recorded.

```bash
bash scripts/ops/2026-10-06/run-custody.sh
```

Related: [#1386](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1386),
[#1421](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1421),
[#1396](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1396).
