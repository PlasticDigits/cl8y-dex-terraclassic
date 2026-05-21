/**
 * Post-sign fee sanity check for extension wallets on LocalTerra (GitLab #127).
 * Mirrored in `patches/@goblinhunt+cosmes+…patch` → `KeplrExtension.js` — keep in sync.
 */

export type AminoFeeLike = {
  amount?: Array<{ denom: string; amount: string }>
}

export const EXTENSION_SIGNED_FEE_UNDERSHOOT_PREFIX =
  'Wallet signed a fee far below what this dApp submitted (GitLab #127).'

function extensionSignedFeeUndershootDetail(expectedUluna: bigint, signedUluna: bigint): string {
  return (
    `${EXTENSION_SIGNED_FEE_UNDERSHOOT_PREFIX} ` +
    'On LocalTerra with Station: disconnect, reconnect, and approve any chain-update prompt. ' +
    'Run `cd frontend-dapp && npm ci` so the cosmes patch is applied, then retry. ' +
    `Expected at least ~${expectedUluna} uluna; wallet returned ~${signedUluna} uluna.`
  )
}

export function ulunaFromAminoFee(fee: AminoFeeLike | undefined): bigint {
  const coin = fee?.amount?.find((c) => c.denom === 'uluna')
  return coin?.amount ? BigInt(coin.amount) : 0n
}

export function isLocalTerraChainId(chainId: string): boolean {
  return chainId.toLowerCase() === 'localterra'
}

/**
 * Returns an error message when the wallet-signed fee is far below the dApp fee, else null.
 */
export function extensionSignedFeeUndershootMessage(
  signedDoc: { fee?: AminoFeeLike },
  expectedFee: AminoFeeLike,
  chainId: string
): string | null {
  if (!isLocalTerraChainId(chainId)) return null

  const expectedUluna = ulunaFromAminoFee(expectedFee)
  if (expectedUluna <= 0n) return null

  if (!signedDoc.fee) {
    return extensionSignedFeeUndershootDetail(expectedUluna, 0n)
  }

  const signedUluna = ulunaFromAminoFee(signedDoc.fee)
  if (signedUluna >= expectedUluna / 2n) return null

  return extensionSignedFeeUndershootDetail(expectedUluna, signedUluna)
}
