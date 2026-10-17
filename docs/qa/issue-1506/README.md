# Issue #1506 — compatible protected direct wrap/unwrap rollout

Source: [DEX #1463](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1463)
and [ustr-cmm #62](https://git.cl8y.com/code/ustr-cmm/pulls/62) (merged `9817dfd`).
Tracker: [#1506](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1506),
split from [#1504](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1504).
Invariants: [QC1459-1–QC1459-10](../../../skills/AGENTS_LOCAL_QUOTE_COVERAGE.md),
[WF1459-1–WF1459-8](https://git.cl8y.com/code/ustr-cmm/src/branch/master/skills/direct-wrapper-floors/SKILL.md).

Goal: the deployed columbus-5 treasury/mapper enforce a reviewed positive
`minimum_receive` for direct wrap/unwrap, and scheduled capability observation
enables the protected signing path in the indexer/frontend. Legacy/unknown
capabilities must keep direct signing blocked.

## Deployed contracts (columbus-5, 2026-10-17)

| Role | Address (unchanged by migrate) | Code ID | Admin |
| --- | --- | --- | --- |
| wrap-mapper | `terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2` | **11729** (was 11574) | DEX 2-of-3 `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7` |
| CMM treasury | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` | **11728** (was 11667) | DEX 2-of-3 `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7` |

Code SHA256 (`data_hash` from LCD `GET /cosmwasm/wasm/v1/code/{id}`):

- treasury 11728: `709B2B1A7026342518BBBE58713690A2E35DFD6127582A12FC9C205FCD33DA76`
- mapper 11729: `B9FC56BAFE0B0C8066ED3F47CE64CC8936C92648352E91304F1632A9FB84C966`

Transactions:

| Step | Tx | Block (UTC) | Code |
| --- | --- | --- | --- |
| store treasury 11728 | `A4C654F3B545002789C1BDB2048F9EE25C7DF597762E54E89FAE5DBAAAA6048E` | 30873663 · 05:38:42 | 0 |
| store mapper 11729 | `B715B896037A8BA3ABD1734A28E643E8EBA90F67314AD2268ADBFE483B2E6480` | 30873666 · 05:39:00 | 0 |
| migrate mapper → 11729 | `4F4DDF3ADEC59206D7ED7FE08081B64C83F1FA25C752303C6FC9A27DAC51449C` | 30873672 · 05:39:36 | 0 |
| migrate treasury → 11728 | `43CA371388BD0127BF7631543192C21A2EAC309822A326985CC204499350A0DD` | 30873676 · 05:39:59 | 0 |

The two store transactions are **not** swaps; they succeeded (`code 0`). The
migrate transactions are what make the capability live. A direct wrap attempted
between the first store and the treasury migrate (or before the ~10 s scheduled
capability poll refreshed the indexer) is refused pre-broadcast and leaves no
on-chain transaction.

## Capability readback

```
{"execution_capabilities":{}}  on mapper    -> {"direct_wrapper_floor_version":1}
{"execution_capabilities":{}}  on treasury  -> {"direct_wrapper_floor_version":1}
```

Live `GET https://indexer.dex.cl8y.com/api/v1/route/solve` for
cUSTC→USTC / cLUNC→LUNC / USTC→cUSTC / LUNC→cLUNC returns
`execution_supported:true`, `floor_protocol_version:1`, `execution_blocker:null`,
and a positive `minimum_receive` (e.g. 10 USTC wrap: est `9800000`, floor
`9310000` at 5% slippage). The historical "Direct wrapping cannot enforce the
reviewed receive floor" blocker is gone.

Config preserved by migrate: mapper `fee_wrap_bps=200`, `fee_unwrap_bps=51`,
`paused=false`; all `denom_wrappers` / `all_denom_mappings` intact.

## Live protected wrap (executed)

`2EF256D3CCFA8FFD94802176AFADA97E523D15F3DB652222F8EBE46AEBF1574F`
(block 30873880 · 2026-10-17T06:00:09Z · code 0), sender
`terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv`:

```
treasury  action=wrap_deposit        denom=uusd amount=10000000
mapper    action=notify_deposit      fee=200000 gross_amount=10000000 mint_amount=9800000
cUSTC     action=mint                amount=9800000 to=terra1hu4...
mapper    action=direct_floor_settled minimum_receive=9310000 received=9800000
```

The protected floor settled and delivered `9,800,000 >= 9,310,000`.

`A32E783FF7456F01DB9D1A1BADB3EDA1D6F3E89BC773735FEB102656973071BB`
(block 30873889 · 06:01:04Z · code 0) is a **second protected wrap**
(`wrap_deposit_with_minimum` uusd `9,525,900` → cUSTC `9,335,382`, floor
`8,868,613`), not an unwrap. `received=9,335,382 >= 8,868,613`.

## Live protected unwrap (executed)

`902DF32EE8EC7F7B91F418814C41CC3C84903C8A8067238588513E2943E5683E`
(block 30874148 · 2026-10-17T06:26:40Z · code 0), sender
`terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv`:

```
cUSTC     action=send               to=mapper amount=19135382
mapper    action=unwrap             denom=uusd fee=97590 gross_amount=19135382 recipient=terra1hu4...
cUSTC     action=burn               amount=19135382 from=mapper
treasury  action=instant_withdraw   amount=19037792 denom=uusd recipient=terra1hu4...
mapper    action=direct_floor_settled minimum_receive=17814615 received=18752226
```

The protected floor settled on the actual recipient native delta and delivered
`18,752,226 >= 17,814,615` after the mapper fee, burn tax and bank deduction.

A pre-broadcast simulation of the same path (1 cUSTC, floor `900000`) also
passed (`gas_used 577065 / 1200000`, `direct_floor_settled received=979977`),
which is how the path was validated before the live broadcast.

## Regression: legacy/unknown capability still blocks

Source-only (no legacy deployment remains): `execution_supported` is derived
exclusively from independently scheduled `execution_capabilities` reads to the
bound mapper (and treasury for wrap). Missing/unknown/stale capabilities keep
`execution_supported:false` and the signing blocker; `assertWrapperOnlySubmit`
and `executeLocalNativeRoute` reject before broadcast. Protected messages bind
the exact reviewed floor and generation; the mapper measures the recipient's
actual balance delta and reverts atomically on a miss. See
[`indexer/src/api/local_wrapper_quote.rs`](../../../indexer/src/api/local_wrapper_quote.rs),
[`indexer/src/api/local_quote/modes.rs`](../../../indexer/src/api/local_quote/modes.rs)
and [`frontend-dapp/src/utils/localWrapperOnlyQuote.ts`](../../../frontend-dapp/src/utils/localWrapperOnlyQuote.ts).

## Ops probe

[`scripts/check-ust1-wrap-ops-health.sh`](../../../scripts/check-ust1-wrap-ops-health.sh)
now reports section `[2c] direct-wrapper floor capability (#1506)` and PASSes
when both contracts advertise version 1. `UST1_OPS_STRICT_CAPABILITY=1` turns a
missing/rolled-back capability into a hard failure so an unnoticed revert
re-blocks direct signing.

## Verify

Read-only (no keys):

```bash
scripts/check-ust1-wrap-ops-health.sh
curl -sS 'https://indexer.dex.cl8y.com/api/v1/route/solve?token_in=uusd&token_out=terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch&amount_in=10000000&pool_only=true&protection_policy=quote_relative_v1&max_spread=0.05'
```

## Acceptance

Both directions are now verified by executed mainnet transactions with positive
wallet floors: protected **wrap** `2EF256D3…` and protected **unwrap**
`902DF32E…`. The deployed mapper/treasury enforce the reviewed `minimum_receive`
via the actual recipient balance delta and revert atomically on a miss. Legacy
and unknown capabilities still block direct signing (source-verified). The
30-minute accessible signing window is in `main` but takes effect only after the
indexer and frontend are redeployed.
