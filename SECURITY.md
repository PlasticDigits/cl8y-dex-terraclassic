# Security contact and responsible disclosure

If you see a **suspicious trade**, an **unexpected balance change**, a **broken or misleading UI state**, or believe you have found a **security vulnerability** in CL8Y DEX (contracts, indexer, or frontend), use the channels below.

## How to report

1. **Preferred — GitLab security report (structured):**  
   [Open a security report issue](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/new?issuable_template=security_report)  
   The template applies the `security` label and prompts for reproduction steps, impact, and affected components.

2. **Email (private):**  
   [contact@ceramicliberty.com](mailto:contact@ceramicliberty.com)

Include, when possible:

- What you observed (tx hash, pair, wallet address, screenshots)
- Steps to reproduce
- Network (local / testnet / mainnet) and approximate time (UTC)
- Whether funds are at immediate risk

## Responsible disclosure

- **Do not** publish exploit details, proof-of-concept code, or step-by-step attack instructions in public issues, social posts, or on-chain messages **before** we have had a chance to investigate.
- We may ask for additional detail under confidentiality while we triage.
- Coordinated public disclosure is welcome after we confirm a fix or agree on a timeline.

## Response window

- **Initial acknowledgement:** within **48–72 hours** (business days) of a report received at the email above or via the GitLab security template.
- **Status updates:** we aim to provide triage severity and next steps within **5 business days** when reproduction is feasible.
- Critical active fund-loss scenarios are prioritized for same-day triage when reports include enough detail to investigate.

## Scope

| In scope | Examples |
|----------|----------|
| Smart contracts | Factory, Pair, Router, fee-discount, hooks |
| Indexer / API | Route solve, order book, token metadata |
| Frontend / wallet UX | Misleading quotes, phishing UI, wrong network |

General product feedback and non-security bugs should use the normal GitLab issue tracker without the `security` label.

## Related documentation

- Threat model and mitigations: [`docs/security-model.md`](docs/security-model.md)
- Contract invariants: [`docs/contracts-security-audit.md`](docs/contracts-security-audit.md)
- Internal operator incident template: [`docs/templates/incident-dex-indexer.md`](docs/templates/incident-dex-indexer.md) (not for end-user reports; per-incident comms templates in [appendix](docs/templates/incident-dex-indexer.md#appendix-communications-templates-sec-g05), SEC-G05)

Tracked as GitLab [**#392**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/392) (SEC-A07).
