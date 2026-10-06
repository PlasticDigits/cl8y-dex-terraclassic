# Agent playbook: cLUNC/USDT seed

Follow [the seed runbook](../docs/runbooks/mint-clunc-usdt-lp.md) and
[oracle invariants](AGENTS_ORACLE_SAFETY.md) when changing
[`mint-clunc-usdt-lp.sh`](../scripts/mint-clunc-usdt-lp.sh).

- Authenticate exact token and quote contracts; symbols cannot establish identity.
- Keep USDT at 18 decimals and never compare its raw amounts in Bash arithmetic.
- Never mint USDT; refuse nonempty pairs rather than repricing them during seed.
- Require fresh independent source quorum at planning and each preparation call.
- Cap freshness at 300 seconds regardless of a looser operator environment value.
- Reject more than 1% movement from the plan; recompute before preparing again.
- Permit manual numeric LUNC/USD overrides only with `DRY_RUN=1`.

Run `bash scripts/qa/test-clunc-usdt-lp.sh` and
`python3 scripts/qa/test-oracle-lp-consumers.py`. Review the synthetic healthy
controls alongside rejection cases. Record unavailable operator checks as SKIP
with the reason, following [#1391 verification](../docs/qa/issue-1391/README.md).
