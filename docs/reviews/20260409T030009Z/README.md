# CL8Y DEX Terra Classic — production review bundle

**UTC folder:** `20260409T030009Z`  
**Last reviewed (bundle sync):** 2026-04-22 — backlog/matrix refreshed vs `main`; use [GitLab issues](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues) for live epics.  
**Repository:** `cl8y-dex-terraclassic` (Terra Classic DEX). **Bridge / cross-chain is out of program scope** for this repo and this review bundle.

| File | Purpose |
|------|---------|
| [REVIEW.md](./REVIEW.md) | Executive summary, architecture inventory, gap narrative, launch paths, answers to review questions |
| [ARCHITECTURE_GAP_MATRIX.md](./ARCHITECTURE_GAP_MATRIX.md) | Capability × mode × completeness matrix |
| [SECURITY_REVIEW.md](./SECURITY_REVIEW.md) | Governance-trusted security analysis with evidence |
| [TEST_GAP_MATRIX.md](./TEST_GAP_MATRIX.md) | Test coverage map and missing scenarios |
| [RELEASE_READINESS_MATRIX.md](./RELEASE_READINESS_MATRIX.md) | Operational / release gates |
| [ISSUE_BACKLOG.md](./ISSUE_BACKLOG.md) | Prioritized GitLab-ready issues by epic |
| [ISSUE_TEMPLATES/](./ISSUE_TEMPLATES/) | Copy-paste issue templates |

**Terminology (this review):**

- **v2 swaps** — TerraSwap-compatible **pool-only** execution: direct pair `Swap` or router `ExecuteSwapOperations` with `hybrid: None` / omitted. No separate on-chain label “v2”; this matches [`docs/architecture.md`](../../architecture.md) compatibility story.
- **Limit orders** — On-chain FIFO book: `PlaceLimitOrder`, `CancelLimitOrder`, pair `orderbook`.
- **Hybrid** — Pattern C: `HybridSwapParams` on pair/router `TerraSwap` operations.
