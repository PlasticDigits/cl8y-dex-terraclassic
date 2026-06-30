# CL8Y DEX — Public security posture

This page is the **user-facing security posture** for the CL8Y DEX interface on Terra Classic. It states what users and integrators should expect at launch, how residual risk is bounded at low TVL, and how operator security requirements **scale with TVL** so attack economics stay unfavorable as the product grows.

**dApp footer:** the interface links here from every page (`LegalFooterNotice` → this document).

**Deeper references (operators / auditors):**

- [Security model](./security-model.md) — governance, treasury, hooks, off-chain trust boundaries
- [Contracts security audit & invariants](./contracts-security-audit.md) — invariant matrix, attack paths, residual risks
- [Launch checklist](./runbooks/launch-checklist.md) — production operator gate

Tracked as checklist item **SEC-A01** (GitLab [#387](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/387)).

## Launch expectations (small TVL)

CL8Y DEX is launching with **intentionally capped expectations** appropriate for early, limited total value locked (TVL):

| Area | What users should expect |
|------|--------------------------|
| **Audit** | CosmWasm contracts are **not yet covered by a formal third-party audit**. In-repo documentation and automated tests exist; they **do not** replace external review. |
| **Economic upside for attackers** | At low TVL, on-chain exploit profit is bounded by pool and escrow balances. Operators treat early mainnet as **limited-blast-radius** until TVL and controls justify higher assurance. |
| **Interface** | The browser dApp is a convenience layer. On-chain contracts enforce swap math and auth; **quotes, logos, and routes** from the indexer are off-chain and must be treated as hints — see [off-chain trust boundaries](./security-model.md#off-chain-trust-boundaries-frontend--indexer). |
| **Governance** | Factory governance can change fees, register hooks, pause pairs, and apply trading blacklists. Users trust **honest governance** for policy, not for day-to-day swaps between unrelated wallets. |
| **Support** | No guaranteed incident response SLA at launch. Use this document and linked runbooks to understand limits before depositing material funds. |

Nothing on this site is financial, investment, legal, or tax advice.

## Audit status

- **Formal third-party audit:** not completed at the time of this writing.
- **In-repo assurance:** [contracts-security-audit.md](./contracts-security-audit.md) documents trust assumptions, invariant rows (P1–L17, etc.), and mapping to `smartcontracts/tests/`.
- **Before high-TVL mainnet:** operators should commission an external audit, remediate findings, and expand monitoring and key-management controls per the TVL ladder below.

## Admin controls users should know

These are **governance-only** unless noted. Malicious use by governance is a residual risk (documented, not an access-control bug).

| Control | Effect |
|---------|--------|
| **Governance key** | Whitelist CW20 code IDs, set pair fees, register hooks, update treasury, trading blacklist, pair-creation fee |
| **Treasury** | Receives swap commissions; misconfiguration is an ops risk |
| **Per-pair pause** | Blocks swaps, new limits, cancels, claims, and book clean until unpaused (maker escrow remains on-chain) |
| **Hooks** | Post-swap callbacks; a reverting hook **fails the whole swap** — register only audited, bounded hooks |
| **Fee-discount registry** | Tier registration, trusted routers, lazy deregistration on insufficient CL8Y balance |
| **Trading blacklist** | Factory can block wallets, tokens, or pairs from protocol interaction |

Details: [security-model.md](./security-model.md).

## Known residual risks

Under **honest governance**, these remain product and ops risks rather than unauthorized-caller bugs:

1. **Malicious or compromised governance** — destructive hooks, pause, fee changes, broken discount registry (users pay full fee if queries fail).
2. **Unaudited or mis-whitelisted CW20 templates** — fee-on-transfer tokens can desync reserves (invariant P2); ops policy forbids whitelisting them.
3. **Indexer / frontend compromise** — malicious but valid routes or logos; mitigated by HTTPS pinning, route display at confirmation, logo allowlist — not eliminated in-browser.
4. **Wasm admin / migration keys** — outside contract crate logic; restrict per [wasm admin runbook](./runbooks/wasm-admin-migration.md).
5. **Oracle / TWAP consumers** — manipulation cheaper on thin pools; see [twap-oracle.md](./twap-oracle.md).

Full matrix: [contracts-security-audit.md § Residual risks](./contracts-security-audit.md#residual-risks-not-bugs-under-trusted-governance).

## Security requirements scale with TVL

Security spend and controls should track **economic attractiveness** to attackers. The goal is to keep expected attacker return negative (cost of exploit + legal/operational risk > recoverable value) at each growth stage.

**Protocol TVL** here means the USD value of assets in CL8Y pools, limit-order escrow, and router-adjacent state that a contract bug could drain in one incident (not market cap or off-chain balances).

| Stage | Protocol TVL (USD) | Typical exploit economics | Indicative annual security spend | Minimum operator posture |
|-------|-------------------|---------------------------|----------------------------------|---------------------------|
| **Bootstrap (low)** | **$0 – $1M** | Max extractable value is bounded by on-chain balances; many historical DEX drains in this band are **$100k–$1M**, while a focused contract exploit typically costs **$50k–$250k** in researcher time, tooling, and capital at risk before profit — so sophisticated attacks often have **negative expected value** at the low end of this band | **$15k–$80k** (in-repo tests, basic monitoring, incident template; formal audit optional only while TVL stays well under **$500k**) | Multisig/DAO governance (no EOA admin); no unaudited hooks; HTTPS indexer; pinned `VITE_*` addresses; public posture (this page) linked from dApp; proactive anomaly thresholds in [anomaly signals runbook](./runbooks/anomaly-signals.md) (SEC-G02, [#435](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/435)) |
| **Growth** | **$1M – $25M** | TVL exceeds the professional research floor; targeted review, indexer MITM, and social engineering become rational. Peer DEX incidents at **$1M–$10M TVL** commonly involve **$500k–$5M** direct loss when a bug is found | **$80k–$300k** (initial external audit before sustained TVL crosses **~$5M**; monitoring; modest bug bounty or disclosed disclosure channel) | External audit (or audit delta on changed bytecode); factory/pair admin-tx monitoring; hook and whitelist change review; rate limits and WAF on indexer |
| **Mature** | **$25M+** | Sustained MEV, governance-capture attempts, and repeat professional campaigns are economically justified; large-protocol DEX failures often exceed **$10M** when TVL is **$50M+** | **$250k–$1M+** (full audit + periodic re-audit, 24/7 paging, substantial bounty — industry practice is roughly **0.25–1% of TVL** per year on mature DeFi) | Multisig threshold and signer diversity; timelocks on sensitive governance actions where feasible; formal key ceremony; TWAP/oracle safeguards on thin pools (see [$100k per-pair floor](./twap-oracle.md)); compliance blacklist runbooks exercised |

### How these bands are set

Ranges are **indicative**, not on-chain limits. They combine:

1. **Attack cost floor** — developing and executing a novel AMM/limit-order exploit is rarely a weekend project; serious attempts usually require tens to hundreds of thousands of dollars in effort before any payout.
2. **Historical DEX loss tiers** — opportunistic / griefing incidents often sit below **$1M**; targeted contract bugs on small-to-mid protocols commonly land **$1M–$10M**; mature-protocol failures can exceed **$10M–$100M** when TVL is large.
3. **When security spend should step up** — complete an external audit and disclosure channel **before** TVL is sustained above **~$5M**; budget for re-audits, paging, and bounty caps that scale with TVL **before** crossing **~$25M**.

**Why this matters:** at bootstrap TVL, the **maximum extractable value** from pools, limit escrow, and router state is often smaller than the cost to find and weaponize a bug — so rational attackers frequently have negative expected value. As TVL rises through growth and mature bands, the same bug class can become profitable; operators must **raise the bar** (audit, monitoring, key hygiene, timelocks) **before** TVL crosses the thresholds above.

This ladder is not automatic on-chain enforcement — it is an **operator commitment** documented for users and integrators. Launch checklist [**#337**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/337) tracks executable gates.

## Verification

Third parties can confirm SEC-A01 without chain access:

1. Open the dApp footer on desktop and mobile — link text **Security and audit docs** is visible without scrolling the main content area on standard viewports.
2. Link target is `docs/security-posture.md` on the default branch (via `DOCS_GITLAB_BASE` in `frontend-dapp/src/utils/constants.ts`).
3. This page covers capped launch expectations, audit disclaimer, admin controls, residual risks, and TVL-scaled requirements with **explicit USD bands** (bootstrap **$0–$1M**, growth **$1M–$25M**, mature **$25M+**).

Frontend unit test: `frontend-dapp/src/components/legal/__tests__/LegalFooterNotice.test.tsx`.
