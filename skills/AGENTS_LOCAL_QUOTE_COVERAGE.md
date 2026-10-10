# Local quote coverage (#1459)

Apply when changing direct native wrapper signing, scheduled tax-wallet admission
or composed exact-output sizing. Issue: [#1459](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1459).
Baseline: [#1458](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1458).
Read [LQ1452](AGENTS_LOCAL_SWAP_QUOTES.md), [RT1381](AGENTS_ROUTE_LOCAL_TAX.md),
[NE1398](AGENTS_NET_EXECUTION_FLOORS.md), [LG1390](AGENTS_ROUTE_LAUNCH_GUARDS.md)
and the [review checklist](../docs/qa/issue-1459/README.md).

| ID | Required behavior |
| --- | --- |
| QC1459-1 | Quote, reference, refresh, preparation and signing read observations only. Requests never discover a wallet, enqueue observation work or accelerate the independently scheduled worker. |
| QC1459-2 | Direct wrapping requires fresh protocol version 1 observations for both treasury and mapper. Direct unwrapping requires the mapper. Missing, malformed, future, stale or unknown capabilities leave estimates nonexecutable. Capability changes bind the generation. |
| QC1459-3 | Sign explicit `wrap_deposit_with_minimum` or `unwrap_with_minimum` variants with the reviewed positive wallet `minimum_receive`. Never attach ignored fields to legacy variants or substitute the pre-bank-tax base floor for the wallet floor. Old contracts must reject the new variant. |
| QC1459-4 | The external mapper checks the recipient's actual balance delta after mint/send in a reply. Insufficient output reverts input, mint/burn, fees, rate usage and transfer atomically. Direct unwrap arithmetic is independent of DEX router readiness; pool-bearing native routes retain their router compatibility check. |
| QC1459-5 | Independently enrolled wallets and bounded CW20 holder pages extend coverage beyond the indexer actor census. Every admitted wallet still requires observed factory blacklist status and per-token identity/cap state. Enrollment or holder presence alone never authorizes execution. |
| QC1459-6 | Actor observations retain their original timestamps. Later pair/config/reserve publications cannot refresh them. Failed reads cannot erase a known block or token-specific protocol role. Record negatives before dependent awaits; in-flight and deeper results recheck current epoch/generation. Only a newer fully admitted explicit false role clears a pending token guard. Original expiry bounds the quote. Capacity exhaustion preserves negative contexts and quarantine; partial holder enumeration cannot clear it. |
| QC1459-7 | Nonzero cooldown remains unsupported until public observations expose last-trade state with exact parity. AutoLP remains unsupported until trigger inventory, approved policy and settlement/reserve effects can be modeled. Manager/tax exemption never waives these restrictions. |
| QC1459-8 | Individual hook deductions use independent floors against the original output. Composed inverse must validate the complete forward path. Refine at the next first-hop pre-hook pool-output event; intermediate delivered output can decrease and cannot be used as a monotone search bound. |
| QC1459-9 | Keep path, hop-probe and whole-route refinement bounds explicit. Exhaustion is unavailable, never a sufficient quote. Successful supported candidates need forward sufficiency and independently checked first-reaching minimality. Preserve gross v1 funding, caps, L9 spread, reference guards and positive floors. |
| QC1459-10 | Run `make verify-issue-1452`, `make verify-issue-1398`, `make verify-issue-1381` and `make verify-issue-1390`, then retest corrections on unchanged sources. Record exact commands/revision/time/results. Unavailable checks may be explicit SKIP with overall PASS WITH SKIPS; executed failures stay FAIL and skipped deployment is never acceptance. |

External protocol source and rollback verification belong to the companion
[ustr-cmm PR #62](https://git.cl8y.com/code/ustr-cmm/pulls/62) linked from the checklist. Source support does not upgrade an
existing treasury/mapper deployment. Coordinate release under
[#1456](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1456), and keep
[#1449](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1449) DeFiLlama scope separate.
