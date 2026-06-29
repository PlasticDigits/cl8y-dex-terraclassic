/**
 * Post-sign fee/gas sanity check for extension wallets on LocalTerra (GitLab #127, #134).
 * Intentionally inactive on mainnet (`columbus-5`, etc.) — see SEC-E08 / GitLab #429.
 * Mirrored in `patches/@goblinhunt+cosmes+…patch` → `KeplrExtension.js` — keep in sync.
 */

export type AminoFeeLike = {
  amount?: Array<{ denom: string; amount: string }>
  gas?: string
}

/** Retail copy surfaced in the UI when the wallet rewrites fees far below the dApp envelope (GitLab #371). */
export const EXTENSION_SIGNED_FEE_UNDERSHOOT_USER_MESSAGE =
  'Transaction fee mismatch. Please reconnect your wallet and try again. If the problem persists, contact support.'

/** Legacy diagnostic prefix — kept for humanizing older throws and console diagnostics (GitLab #127). */
export const EXTENSION_SIGNED_FEE_UNDERSHOOT_PREFIX =
  'Wallet signed a fee far below what this dApp submitted (GitLab #127).'

/** Retail copy for UI — keep diagnostics in guard/patch throws only (GitLab #371). */
export const EXTENSION_SIGNED_FEE_USER_MESSAGE =
  'Transaction fee mismatch. Please reconnect your wallet and try again. If the problem persists, contact support.'

/** Minimum signed/expected ratio (percent). 50% allowed fee-only drift (#127); 95% blocks ~23 vs ~36 LUNC swap repro (#134). */
export const EXTENSION_SIGNED_FEE_MIN_PERCENT = 95n

function extensionSignedFeeUndershootDetail(
  expectedUluna: bigint,
  expectedGas: bigint,
  signedUluna: bigint,
  signedGas: bigint
): string {
  const gasNote =
    expectedGas > 0n && signedGas > 0n && signedGas < expectedGas
      ? ` Expected gas at least ~${expectedGas}; wallet returned ~${signedGas}.`
      : ''
  return (
    `${EXTENSION_SIGNED_FEE_UNDERSHOOT_PREFIX} ` +
    'On LocalTerra with Station: disconnect, reconnect, and approve any chain-update prompt. ' +
    'Run `cd frontend-dapp && npm ci` so the cosmes patch is applied, then retry. ' +
    `Expected at least ~${expectedUluna} uluna; wallet returned ~${signedUluna} uluna.${gasNote}`
  )
}

export function ulunaFromAminoFee(fee: AminoFeeLike | undefined): bigint {
  const coin = fee?.amount?.find((c) => c.denom === 'uluna')
  return coin?.amount ? BigInt(coin.amount) : 0n
}

export function gasFromAminoFee(fee: AminoFeeLike | undefined): bigint {
  const g = fee?.gas
  if (g == null || g === '') return 0n
  try {
    return BigInt(g)
  } catch {
    return 0n
  }
}

export function isLocalTerraChainId(chainId: string): boolean {
  return chainId.toLowerCase() === 'localterra'
}

function meetsMinSignedRatio(signed: bigint, expected: bigint): boolean {
  if (expected <= 0n) return true
  if (signed <= 0n) return false
  return signed * 100n >= expected * EXTENSION_SIGNED_FEE_MIN_PERCENT
}

/**
 * Returns an error message when the wallet-signed fee or gas is far below the dApp envelope, else null.
 */
export function extensionSignedFeeUndershootMessage(
  signedDoc: { fee?: AminoFeeLike },
  expectedFee: AminoFeeLike,
  chainId: string,
  expectedGasLimit?: bigint
): string | null {
  if (!isLocalTerraChainId(chainId)) return null

  const expectedUluna = ulunaFromAminoFee(expectedFee)
  const expectedGas = expectedGasLimit ?? gasFromAminoFee(expectedFee)
  if (expectedUluna <= 0n && expectedGas <= 0n) return null

  if (!signedDoc.fee) {
    console.warn('[extensionSignedFeeGuard]', extensionSignedFeeUndershootDetail(expectedUluna, expectedGas, 0n, 0n))
    return EXTENSION_SIGNED_FEE_UNDERSHOOT_USER_MESSAGE
  }

  const signedUluna = ulunaFromAminoFee(signedDoc.fee)
  const signedGas = gasFromAminoFee(signedDoc.fee)

  if (!meetsMinSignedRatio(signedGas, expectedGas) || !meetsMinSignedRatio(signedUluna, expectedUluna)) {
    console.warn(
      '[extensionSignedFeeGuard]',
      extensionSignedFeeUndershootDetail(expectedUluna, expectedGas, signedUluna, signedGas)
    )
    return EXTENSION_SIGNED_FEE_UNDERSHOOT_USER_MESSAGE
  }

  return null
}
