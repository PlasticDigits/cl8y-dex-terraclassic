import { CosmosTxV1beta1AuthInfo as AuthInfo, type CosmosTxV1beta1Fee as Fee } from '@goblinhunt/cosmes/protobufs'
import { OFFLINE_SWAP_GAS_CAP } from './offlineSwapBudget'
import { estimateFeeUlunaAmountForGasLimit } from './terraGas'

export const SIGNED_OFFLINE_SWAP_FEE_MISMATCH =
  'Wallet changed the reviewed network fee. Review the quote and try again.'

/** Local reviewed execution only: split APIs expose signed Fee before any RPC broadcast.
 * Preserve the separate historical chain-scoped extension guard and preferNoSetFee.
 * Atomic WalletConnect post has no pre-broadcast signed bytes and cannot use this check. */
export function assertSignedOfflineSwapFee(
  authInfoBytes: Uint8Array,
  requested: Fee
): {
  requestedGas: bigint
  signedGas: bigint
  requestedUluna: bigint
  signedUluna: bigint
} {
  try {
    const fee = AuthInfo.fromBinary(authInfoBytes).fee
    if (!fee || fee.amount.length !== 1 || fee.amount[0].denom !== 'uluna' || !/^\d{1,39}$/.test(fee.amount[0].amount))
      throw new Error()
    const requestedUluna = BigInt(requested.amount.find((coin) => coin.denom === 'uluna')!.amount)
    const signedUluna = BigInt(fee.amount[0].amount)
    const signedGas = fee.gasLimit
    if (
      signedGas !== requested.gasLimit ||
      signedGas > BigInt(OFFLINE_SWAP_GAS_CAP) ||
      signedUluna !== requestedUluna ||
      (fee.payer ?? '') !== (requested.payer ?? '') ||
      (fee.granter ?? '') !== (requested.granter ?? '') ||
      signedUluna < estimateFeeUlunaAmountForGasLimit(Number(signedGas))
    )
      throw new Error()
    return { requestedGas: requested.gasLimit, signedGas, requestedUluna, signedUluna }
  } catch {
    throw new Error(SIGNED_OFFLINE_SWAP_FEE_MISMATCH)
  }
}
