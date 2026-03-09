# QA Onboarding

## Prerequisites

- **git** and **gh** CLI installed and authenticated
- **Node.js 24** (use `nvm use` in the repo root)
- A Terra Classic wallet (Station extension) with testnet LUNC
- Access to the GitHub repository

## Quick Start

```bash
git clone <repo-url> && cd cl8y-dex-terraclassic
git config core.hooksPath .githooks
cd frontend-dapp && npm ci
VITE_NETWORK=testnet npm run dev
```

Open `http://localhost:3000` and connect your wallet.

## What to Test

### Swap Flow
1. Connect wallet
2. Select input/output tokens from the dropdown
3. Enter an amount — verify the estimated output updates
4. Click Swap — confirm the transaction in Station
5. Verify balances updated correctly
6. Verify the fee was deducted (check treasury balance or tx events)

### Pool / Liquidity
1. Navigate to `/pool`
2. Select a pair
3. Add liquidity — enter both token amounts, confirm
4. Verify LP tokens received
5. Remove liquidity — enter LP amount, confirm
6. Verify both tokens returned

### Create Pair
1. Navigate to `/pool/create`
2. Enter two valid CW20 token addresses
3. Submit — confirm the Factory transaction
4. Verify the new pair appears in the pool list

## Wallet Matrix

| Wallet          | Platform    | Priority |
|-----------------|-------------|----------|
| Station (ext)   | Chrome      | P0       |
| Station (ext)   | Firefox     | P1       |
| Station (mobile)| iOS/Android | P1       |

## CLI Workflow

### File a bug
```bash
./scripts/qa/new-bug.sh "swap fails with zero amount"
# or with evidence:
./scripts/qa/new-bug.sh --evidence /path/to/screenshot.png "swap fails with zero amount"
```

### File a test pass
```bash
./scripts/qa/new-test-pass.sh
```

## Security Escalation

If you discover a potential security issue (e.g., unauthorized access, fund loss, contract exploit):

1. **Do NOT** file a public GitHub issue
2. Contact `@PlasticDigits` directly via a private channel
3. Include: steps to reproduce, affected contract/function, potential impact

## Device Checklist

- [ ] Desktop Chrome (latest)
- [ ] Desktop Firefox (latest)
- [ ] Mobile Safari (iOS)
- [ ] Mobile Chrome (Android)
- [ ] Tablet (either platform)
