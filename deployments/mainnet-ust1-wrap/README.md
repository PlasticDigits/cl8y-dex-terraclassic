# Mainnet UST1 + wrap registry (columbus-5)

Canonical **Phase 2–4** address pack for production ops hardening ([GitLab **#503**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/503), parent [#502](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/502)).

| File | Purpose |
|------|---------|
| [`REGISTRY.md`](./REGISTRY.md) | Single address table (no secrets) + Coolify keys + cross-repo mirrors |
| [`coolify.env.example`](./coolify.env.example) | Frontend `VITE_*` build-args for `dex.cl8y.com` |

**Ops runbook:** [`docs/runbooks/ust1-wrap-production-ops.md`](../../docs/runbooks/ust1-wrap-production-ops.md)
**Agent playbook:** [`skills/AGENTS_UST1_WRAP_PRODUCTION_OPS.md`](../../skills/AGENTS_UST1_WRAP_PRODUCTION_OPS.md)
**Verify:** `make verify-issue-503` (optional `VERIFY503_MAINNET=1` for live LCD checks)

Do **not** commit operator secrets, key material, or filled pager credentials here.
