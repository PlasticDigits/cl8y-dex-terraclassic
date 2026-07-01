# Production governance multisig

Canonical on-chain address for **governance**, **wasm contract admin**, and **contract upgrades** across the CL8Y DEX stack (factory, router, pair, fee-discount, treasury, wrap-mapper, hooks).

| Role | Address |
|------|---------|
| **Governance / admin / upgrade** | `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7` |

## Usage

- **Factory `config.governance`** — fees, hooks, pause, whitelist, blacklist, treasury pointer
- **Fee-discount `config.governance`** — tier registry, trusted routers
- **Treasury / wrap-mapper `governance`** — pause, config updates
- **Wasm `--admin`** on every instantiate — migration and `set-contract-admin`

Shell scripts source [`scripts/lib/governance-multisig.sh`](../../scripts/lib/governance-multisig.sh) as `GOVERNANCE_MULTISIG_ADDR`.

Mainnet deploy (defaults to this address):

```bash
make deploy-mainnet
# or: ./scripts/deploy-dex-mainnet.sh
```

Override only for non-production rehearsal:

```bash
./scripts/deploy-dex-mainnet.sh terra1other...
```

## LocalTerra

Local deploy (`make deploy-local`) uses the well-known dev account `test1` (`terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v`) — not the production multisig.

## Related

- [Key custody runbook](../runbooks/key-custody.md) (SEC-B10)
- [Security model § Governance](../security-model.md#governance-keys)
- [Deployment guide](../deployment-guide.md)
