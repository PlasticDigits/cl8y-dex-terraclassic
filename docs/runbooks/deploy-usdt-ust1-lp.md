# USDT-Bridge-Debt and 10,000 USDT / 10,000 UST1 LP

A bridged-mint borrow of asset XXX mints XXX for liquidity and the same amount of `XXX-Bridge-Debt` to the debtor. The first market is USDT for the CMM LP.

User bridge withdrawals stay on the bridge. Those mints are backed by a remote lock and do not create a Bridge-Debt balance.

Issue #1393 hardening follows [BD1393-1–BD1393-10](../../skills/AGENTS_BRIDGE_DEBT_MARKET_PINS.md).
Use [the agent checklist](../qa/issue-1393/README.md) and `make verify-issue-1393`.
These are supply-accounting controls, not an enforced collateral guarantee.
Any $1 receipt value is a display convention; it is not a peg or redemption price.

## Contracts

| Crate | Role |
|-------|------|
| [`bridge-debt-token`](../../smartcontracts/contracts/bridge-debt-token) | CW20 receipt. Symbol and name are set at instantiate (`USDT-Bridge-Debt`, later `XXX-Bridge-Debt`). Only the minter can mint, and only to the debtor. Holders cannot transfer or burn. The minter burns the debtor's balance on repay. |
| [`bridge-debt`](../../smartcontracts/contracts/bridge-debt) | Owner-gated issuer. `Issue` mints the bridged token to the LP recipient and the receipt to the debtor in one transaction. A CW20 `Send` with `{"repay":{}}` burns both by the same amount. |

`USDT-Bridge-Debt` is 16 characters. Stock CW20 tickers stop at 12, so this token allows symbols of 3–32 characters in `[A-Za-z0-9-]`.

## USDT market

| | |
|--|--|
| Underlying | Bridged USDT `terra1z0xe7t5ymmltg4vju8tghkq0pewy4et548ta23nlu9zxtl950uyqkv8mv4` (18 decimals, code `10184`) |
| Receipt | `USDT-Bridge-Debt`, 18 decimals, debtor = CMM treasury `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` |
| USDT minters | Primary `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7` (2-of-3). Additional: Terra bridge `terra18m02l2f43c2dagqnz3kfccpgz9pzzz5hk9l5mh5wvr6dcvv47zfqdfs7la` |
| First LP borrow | `10000000000000000000000` base units (10,000 USDT) |

The issuer must be the receipt's sole minter, and the 2-of-3 must `AddMinter` the issuer on USDT before `Issue` can succeed. The issuer owner is that same 2-of-3.

## Messages

Instantiate the receipt after the issuer exists:

```json
{
  "name": "USDT-Bridge-Debt",
  "symbol": "USDT-Bridge-Debt",
  "decimals": 18,
  "minter": "<bridge-debt issuer>",
  "debtor": "terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2",
  "cap": null
}
```

Then, as issuer owner:

```json
{ "set_debt_code_id": { "code_id": "<bridge-debt-token code id>" } }
```

Approve the reviewed underlying implementation separately from factory listing.
Require the live LCD code ID to match the independently reviewed
`REVIEWED_USDT_CODE_ID` (default `10184` for the existing USDT template).
An observed implementation change needs a separate code review before overriding
that input; the script rejects the mismatch before any transaction. `10184` is
not an ID to reuse on another chain. Multi-minter templates must
provide a complete bounded `Minters` inventory (at most 30 entries), while approved
single-minter implementations pin `Minter` address and cap without that extension.

```json
{ "approve_underlying_code": { "code_id": 10184, "multi_minter": true } }
```

```json
{
  "register_market": {
    "underlying": "terra1z0xe7t5ymmltg4vju8tghkq0pewy4et548ta23nlu9zxtl950uyqkv8mv4",
    "debt_token": "<USDT-Bridge-Debt>"
  }
}
```

Registration is issuance-disabled until explicit finite raw-unit limits exist:

```json
{
  "configure_market": {
    "underlying": "terra1z0xe7t5ymmltg4vju8tghkq0pewy4et548ta23nlu9zxtl950uyqkv8mv4",
    "limits": {
      "max_per_issue": "10000000000000000000000",
      "max_outstanding": "10000000000000000000000",
      "epoch_seconds": 86400,
      "max_per_epoch": "10000000000000000000000"
    },
    "paused": false
  }
}
```

All caps must be positive. Epochs are fixed timestamp buckets, not rolling windows.
Once configured, the duration cannot change; changing caps or pause does not reset
consumed quota. Repayment reduces outstanding but does not refund epoch quota.

```json
{
  "issue": {
    "underlying": "terra1z0xe7t5ymmltg4vju8tghkq0pewy4et548ta23nlu9zxtl950uyqkv8mv4",
    "recipient": "<LP seeding address>",
    "amount": "10000000000000000000000"
  }
}
```

`recipient` receives the USDT that will be paired. The CMM receives the receipt either way. Repay by sending USDT to the issuer with hook `{"repay":{}}` (`eyJyZXBheSI6e319` in base64). The repay amount cannot exceed outstanding debt. A plain USDT transfer does not reduce the debt; the owner can `Recover` those tokens.

Registering another asset uses a new `XXX-Bridge-Debt` instance and the same issuer. Each receipt has one underlying.

## Code upgrades, migration and owner transfer

Every market retains its own receipt/underlying code IDs and exact primary minter,
cap and sorted additional-minter inventory. Rotating `SetDebtCodeId` or approving
another code affects future registrations; live code or minter drift fails closed
for Issue, Repay and Recover. The script refuses mismatched existing pins or limits
and never refreshes them automatically.

For a reviewed code-only child upgrade, approve the target underlying implementation
and call `propose_market_upgrade` with `underlying`, `debt_code_id` and
`underlying_code_id`. Pending proposals disable Issue. A signer who is both child
wasm admin and issuer owner should batch child `MsgMigrateContract` messages and
the issuer `accept_market_upgrade` execute into one Cosmos transaction. Acceptance
checks exact target code IDs, unchanged debtor, decimals, minter/cap inventory,
receipt supply/accounting and underlying supply. Valid Repay while pending adjusts
the proposal's supply snapshot by its burn; unrelated underlying mint/burn makes
acceptance stale and needs `cancel_market_upgrade` followed by a reviewed new
proposal. Cancel does not repair code drift or implicitly accept new implementations.

Issuer migration verifies cw2 contract name and supported source versions. A legacy
market requires an explicit reviewed `legacy_pins` entry, not automatic trust in
whatever code happens to be live:

```json
{
  "legacy_pins": [{
    "underlying": "<underlying address>",
    "pins": {
      "debt_code_id": 123,
      "underlying_code_id": 456,
      "multi_minter": true,
      "minter": "<primary minter>",
      "minter_cap": null,
      "additional_minters": ["<issuer>", "<other approved minter>"]
    }
  }]
}
```

Supply/accounting and identity checks must pass for every market before migrated
state is written. Already-pinned supported state can migrate with
`{"legacy_pins":[]}`. Receipt migrations also reject wrong-contract and newer source
versions. `update_owner` proposes a recipient; that recipient must call
`accept_owner`. The current owner may call `cancel_owner_transfer` before acceptance.

The owner still controls reviewed code approvals, future defaults, finite caps,
pause, upgrade proposals and recovery of accidental underlying deposits. Child
wasm admins can migrate underlying/receipt contracts; the issuer wasm admin can
replace the issuer itself. Other legitimate underlying minters can create or burn
supply outside this debt ledger. These residual powers and bridge custody trust
remain explicit. Pause and issuance caps leave valid consistent Repay/Recover
available; Recover never lowers debt. Receipt holders cannot transfer, burn or
replace the issuer minter.

## 10,000 USDT / 10,000 UST1 LP

[`scripts/deploy-usdt-ust1-lp.sh`](../../scripts/deploy-usdt-ust1-lp.sh) stores the two contracts, then:

1. `multisig_2of3` (`multisig1` + `multisig2`) adds the issuer as a USDT minter for a new market, approves the independently reviewed USDT implementation as multi-minter, registers the market and configures finite 10,000 USDT per-issue/outstanding/86400-second epoch caps before `Issue` to `cl8y2_admin`. The matching `USDT-Bridge-Debt` is minted to the CMM inside that same transaction. Resume skips already-issued debt, validates an existing market before setup mutations, and refuses partial debt, pauses, pending upgrades or differing pins/limits instead of silently widening issuance.
2. `cl8y2_admin` mints 10,000 UST1 to itself, creates the factory pair, and provides liquidity to itself.
3. `cl8y2_admin` transfers the LP token to the CMM treasury.

```bash
BUILD_WASM=1 ./scripts/deploy-usdt-ust1-lp.sh
USDT_UST1_LP_YES=1 ./scripts/deploy-usdt-ust1-lp.sh
```

The first command builds wasm with [`smartcontracts/scripts/optimize.sh`](../../smartcontracts/scripts/optimize.sh) and prints the plan. The second broadcasts. It asks for the keyring passphrase once. Resume state is `deployments/usdt-ust1-lp/state.env` (not committed).

A public-RPC 503 during gas simulation is retried (`TERRAD_HOST_TX_RETRIES`, default 8). If wasm-store simulation stays unavailable, that store is broadcast with `--gas ${STORE_GAS_LIMIT:-25000000}` so the node does not have to simulate the wasm.
