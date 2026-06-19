# Governance emergency controls rehearsal evidence (template)

Copy this table into a GitLab comment on the **launch tracking issue** ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)) or [#397](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/397) after a successful rehearsal.

| Field | Value |
|-------|-------|
| Checklist | SEC-B09 |
| Network | _e.g. `rebel-2` testnet / staging name_ |
| Factory | _factory contract address_ |
| Governance multisig | _multisig address + threshold, e.g. 3-of-5_ |
| Pair exercised | _pair contract used for pause/unpause_ |
| Blacklist target | _wallet address blacklisted then restored_ |
| Rehearsal UTC | _ISO-8601 timestamp_ |
| Signing flow | _e.g. terrad multisign / Safe / DAO proposal #_ |
| Operators | _names or handles_ |

## Operations (in order)

| Step | Factory message | Tx hash |
|------|-----------------|---------|
| 1 | `SetPairPaused { paused: true }` | _hash_ |
| 2 | `BlacklistWallet` | _hash_ |
| 3 | `SetPairPaused { paused: false }` | _hash_ |
| 4 | `UnblacklistWallet` | _hash_ |

## On-chain verification

- [ ] Pair `is_paused` query toggled `true` then `false`
- [ ] Factory `blacklist_check.wallet_blacklisted` toggled `true` then `false`
- [ ] LCD/explorer links attached for each tx hash

## Launch gate

Link this comment from the Phase 5 go/no-go sign-off on [#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391). See [`docs/runbooks/governance-emergency-rehearsal.md`](../runbooks/governance-emergency-rehearsal.md).
