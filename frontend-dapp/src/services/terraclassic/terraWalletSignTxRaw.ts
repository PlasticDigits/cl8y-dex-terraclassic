import { Tx } from '@goblinhunt/cosmes/client'
import { signDirect } from '@goblinhunt/cosmes/codec'
import {
  CosmosTxV1beta1AuthInfo as ProtoAuthInfo,
  CosmosTxV1beta1TxRaw as ProtoTxRaw,
} from '@goblinhunt/cosmes/protobufs'
import type { ConnectedWallet, UnsignedTx } from '@goblinhunt/cosmes/wallet'
import { WalletError, WalletName, WalletType } from '@goblinhunt/cosmes/wallet'
import type { CosmosTxV1beta1Fee as Fee } from '@goblinhunt/cosmes/protobufs'
import { toPlainAminoStdSignDoc } from '@/utils/terraAminoSignDoc'
import {
  extensionSignedFeeUndershootMessage,
  EXTENSION_SIGNED_FEE_MIN_PERCENT,
  isLocalTerraChainId,
  type AminoFeeLike,
  ulunaFromAminoFee,
} from '@/utils/extensionSignedFeeGuard'
import { txHashFromTxRaw } from '@/utils/terraTxHash'
import { walletIsNanoLedger } from '@/services/terraclassic/keplrExtensionConfig'

const EXTENSION_SIGN_OPTIONS = {
  preferNoSetFee: true,
  preferNoSetMemo: true,
} as const

type KeplrSignExtension = {
  signAmino: (
    chainId: string,
    address: string,
    doc: unknown,
    options?: typeof EXTENSION_SIGN_OPTIONS
  ) => Promise<{ signed: { fee?: AminoFeeLike; sequence: string }; signature: { signature: string } }>
  signDirect: (
    chainId: string,
    address: string,
    doc: unknown,
    options?: typeof EXTENSION_SIGN_OPTIONS
  ) => Promise<{ signed: { bodyBytes: Uint8Array; authInfoBytes: Uint8Array }; signature: { signature: string } }>
}

type SplittableWallet = ConnectedWallet & {
  useAmino?: boolean
  isNanoLedger?: boolean
  ext?: KeplrSignExtension
  wc?: KeplrSignExtension
  privateKey?: Uint8Array
  keyType?: 'secp256k1' | 'ethsecp256k1'
}

function gasLimitFromProtobufFee(fee: Fee): bigint {
  return fee?.gasLimit ? BigInt(fee.gasLimit) : 0n
}

function meetsMinSignedRatio(signed: bigint, expected: bigint): boolean {
  if (expected <= 0n) return true
  if (signed <= 0n) return false
  return signed * 100n >= expected * EXTENSION_SIGNED_FEE_MIN_PERCENT
}

function assertExtensionSignedDirectFeeMeetsExpected(
  signed: { authInfoBytes: Uint8Array },
  expectedFee: Fee,
  chainId: string
): void {
  if (!isLocalTerraChainId(chainId)) return

  const expectedUluna = ulunaFromAminoFee({
    amount: expectedFee.amount.map((c) => ({ denom: c.denom, amount: c.amount })),
    gas: expectedFee.gasLimit.toString(),
  })
  const expectedGas = gasLimitFromProtobufFee(expectedFee)
  if (expectedUluna <= 0n && expectedGas <= 0n) return

  if (!signed.authInfoBytes?.length) {
    throw new Error(extensionSignedFeeUndershootMessage({ fee: undefined }, { amount: [], gas: '0' }, chainId)!)
  }

  const auth = ProtoAuthInfo.fromBinary(signed.authInfoBytes)
  const coin = auth.fee?.amount?.find((c) => c.denom === 'uluna')
  const signedUluna = coin?.amount ? BigInt(coin.amount) : 0n
  const signedGas = auth.fee?.gasLimit ? BigInt(auth.fee.gasLimit) : 0n

  if (!meetsMinSignedRatio(signedGas, expectedGas) || !meetsMinSignedRatio(signedUluna, expectedUluna)) {
    const aminoExpected = {
      amount: expectedFee.amount.map((c) => ({ denom: c.denom, amount: c.amount })),
      gas: expectedFee.gasLimit.toString(),
    }
    const undershoot = extensionSignedFeeUndershootMessage(
      { fee: { amount: [{ denom: 'uluna', amount: signedUluna.toString() }], gas: signedGas.toString() } },
      aminoExpected,
      chainId,
      expectedGas
    )
    if (undershoot) throw new Error(undershoot)
  }
}

/**
 * Split-path amino vs signDirect (GitLab #567).
 * Station / Cosmostation always amino (#208). Keplr Ledger (`isNanoLedger` / `useAmino`) never signDirect.
 * Software Keplr stays signDirect unless the wallet object already says amino (K567-1, K567-2).
 */
export function walletUsesAmino(wallet: ConnectedWallet): boolean {
  const w = wallet as SplittableWallet
  if (w.id === WalletName.STATION || w.id === WalletName.COSMOSTATION) {
    return true
  }
  if (walletIsNanoLedger(w)) {
    return true
  }
  if (typeof w.useAmino === 'boolean') {
    return w.useAmino
  }
  return false
}

/** Wallets that sign locally then RPC-broadcast (not atomic WC `post`). */
export function walletSupportsSplitSignBroadcast(wallet: ConnectedWallet): boolean {
  const w = wallet as SplittableWallet
  // dev MnemonicWallet sets id 'mnemonic', outside the cosmes WalletName enum — compare as string.
  if ((w.id as string) === 'mnemonic' || w.privateKey) return true
  if (w.ext?.signAmino || w.ext?.signDirect) return true
  if (w.wc?.signAmino || w.wc?.signDirect) return true
  return false
}

export function bumpWalletCachedSequence(wallet: ConnectedWallet, signedSequence: bigint): void {
  const w = wallet as unknown as { sequence?: bigint }
  if (w.sequence !== undefined) {
    w.sequence = signedSequence + 1n
  }
}

export type SignedTerraTxRaw = {
  txRaw: ProtoTxRaw
  txHash: string
  sequence: bigint
}

export type SignTerraTxRawOptions = {
  /**
   * When true, use the wallet's cached account sequence (after code-32 expected sequence applied).
   * Default refreshes from chain so concurrent signers do not reuse a stale page-load sequence ([GitLab #499](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/499)).
   */
  useCachedSequence?: boolean
}

/**
 * Sign execute-contract msgs without broadcasting — enables post-sign recovery when RPC hangs (GitLab #359).
 * Account sequence is refreshed from chain unless `useCachedSequence` is set (#499).
 */
export async function signTerraTxRaw(
  wallet: ConnectedWallet,
  unsignedTx: UnsignedTx,
  fee: Fee,
  options?: SignTerraTxRawOptions
): Promise<SignedTerraTxRaw> {
  const w = wallet as SplittableWallet
  const { accountNumber, sequence } = await wallet.getAuthInfo(options?.useCachedSequence === true)

  const tx = new Tx({
    chainId: wallet.chainId,
    pubKey: wallet.pubKey,
    msgs: unsignedTx.msgs,
  })

  const params = {
    accountNumber,
    sequence,
    fee,
    memo: unsignedTx.memo,
    timeoutHeight: unsignedTx.timeoutHeight,
  }

  let txRaw: ProtoTxRaw

  if (w.privateKey) {
    const doc = tx.toSignDoc(params)
    const signature = signDirect(doc, w.privateKey, w.keyType ?? 'secp256k1')
    txRaw = tx.toSignedDirect(doc, signature)
  } else {
    const signer = w.ext ?? w.wc
    if (!signer) {
      throw new Error('Wallet does not support offline transaction signing')
    }

    if (walletUsesAmino(w)) {
      const stdDoc = toPlainAminoStdSignDoc(tx.toStdSignDoc(params))
      const { signed, signature } = await WalletError.wrap(
        signer.signAmino(wallet.chainId, wallet.address, stdDoc, EXTENSION_SIGN_OPTIONS)
      )
      const undershoot = extensionSignedFeeUndershootMessage(signed, stdDoc.fee ?? {}, wallet.chainId)
      if (undershoot) throw new Error(undershoot)
      txRaw = tx.toSignedAmino(signed, signature.signature)
    } else {
      const signDoc = tx.toSignDoc(params)
      const { signed, signature } = await WalletError.wrap(
        signer.signDirect(wallet.chainId, wallet.address, signDoc, EXTENSION_SIGN_OPTIONS)
      )
      assertExtensionSignedDirectFeeMeetsExpected(signed, fee, wallet.chainId)
      txRaw = tx.toSignedDirect(signed, signature.signature)
    }
  }

  const txHash = await txHashFromTxRaw(txRaw)
  return { txRaw, txHash, sequence }
}

/** Station / LuncDash WC v1 — atomic `post` with no separate sign step. */
export function isAtomicWalletConnectPost(wallet: ConnectedWallet): boolean {
  return (
    wallet.type === WalletType.WALLETCONNECT && (wallet.id === WalletName.STATION || wallet.id === WalletName.LUNCDASH)
  )
}
