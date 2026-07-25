import { MsgExecuteContract, RpcClient } from '@goblinhunt/cosmes/client'
import type { ConnectedWallet } from '@goblinhunt/cosmes/wallet'
import type { UnsignedTx } from '@goblinhunt/cosmes/wallet'
import { WalletName, WalletType } from '@goblinhunt/cosmes/wallet'
import { prepareStationExtensionForTerraClassicSign } from '@/services/terraclassic/stationExtensionConfig'
import { estimateTerraClassicFeeForEntries } from '@/services/terraclassic/terraClassicFeeEstimate'
import { pollTxUntilRecoveryDeadline } from '@/services/terraclassic/terraTxRecoveryPoll'
import {
  bumpWalletCachedSequence,
  isAtomicWalletConnectPost,
  signTerraTxRaw,
  type SignTerraTxRawOptions,
  walletSupportsSplitSignBroadcast,
} from '@/services/terraclassic/terraWalletSignTxRaw'
import { planAccountSequenceRetry } from '@/utils/terraAccountSequence'
import { resolveTerraTxRecoveryDeadlineUnix } from '@/utils/terraMsgDeadline'
import { tryHumanizeTerraTxMessage } from '@/utils/humanizeTerraTxError'
import {
  isTerraTxTimeoutMessage,
  TERRA_TX_BROADCAST_TIMEOUT_MESSAGE,
  TERRA_TX_BROADCAST_TIMEOUT_MS,
  TERRA_TX_POLL_TIMEOUT_MESSAGE,
  TERRA_TX_POLL_TIMEOUT_MS,
} from '@/utils/terraTxTimeout'
import { withPromiseTimeout } from '@/utils/withPromiseTimeout'
import { buildTerraClassicFee } from './terraGas'
import { getTerraBroadcastScopeOptions } from './terraBroadcastScope'
import { withTerraWalletSignLock } from './terraWalletSignLock'

export type TerraExecuteContractEntry = {
  contract: string
  msg: Record<string, unknown>
  coins?: Array<{ denom: string; amount: string }>
}

export type TerraBroadcastPhase = 'signing' | 'broadcasting' | 'confirming' | 'recovering'

export type TerraBroadcastPhaseChangeContext = {
  txHash?: string
}

export type TerraBroadcastOptions = {
  onPhaseChange?: (phase: TerraBroadcastPhase, ctx?: TerraBroadcastPhaseChangeContext) => void
}

/** CheckTx / mempool rejections — tx never entered the mempool; safe to fail without recovery. */
function isDefiniteBroadcastRejection(error: unknown): boolean {
  if (!(error instanceof Error)) return false
  const msg = error.message
  return (
    /account sequence mismatch|incorrect account sequence/i.test(msg) ||
    /signature verification failed|pubkey does not match/i.test(msg) ||
    /invalid chain[- ]?id|wrong chain/i.test(msg) ||
    /tx too large|memo too large/i.test(msg) ||
    /failed to decode tx|decode tx|invalid transaction/i.test(msg) ||
    /insufficient fees?|minimum fee|auth info validation|signatures validation/i.test(msg)
  )
}

/**
 * After sign, ambiguous RPC failures may still have submitted the tx — poll until deadline.
 * Definite CheckTx rejections skip recovery so the user gets the original error promptly.
 */
function shouldRecoverPostSignBroadcast(error: unknown): boolean {
  return !isDefiniteBroadcastRejection(error)
}

function handleBroadcastError(error: unknown): Error {
  if (error instanceof Error) {
    const errorMessage = error.message

    if (isTerraTxTimeoutMessage(errorMessage)) {
      return error
    }

    const human = tryHumanizeTerraTxMessage(errorMessage)
    if (human) {
      return new Error(human)
    }

    if (/extension popup was closed/i.test(errorMessage) && !/explicitly|you rejected/i.test(errorMessage)) {
      return new Error(
        'Station closed the signing popup before the transaction completed. Disconnect and reconnect Station, approve any Terra Classic network update, then retry. If this persists, update the Station extension.'
      )
    }

    if (
      errorMessage.includes('User rejected') ||
      errorMessage.includes('user rejected') ||
      errorMessage.includes('User denied') ||
      errorMessage.includes('user denied')
    ) {
      return new Error('Transaction rejected by user')
    }

    if (
      errorMessage.includes('Failed to fetch') ||
      errorMessage.includes('NetworkError') ||
      errorMessage.includes('network')
    ) {
      return new Error(`Network error: ${errorMessage}. Please check your internet connection and try again.`)
    }

    return new Error(`Transaction failed: ${errorMessage}`)
  }

  return new Error(`Transaction failed: ${String(error)}`)
}

async function pollTxWithTimeout(
  wallet: ConnectedWallet,
  txHash: string
): Promise<{ txResponse: { code: number; rawLog?: string; logs?: Array<{ log?: string }> } }> {
  return withPromiseTimeout(wallet.pollTx(txHash), TERRA_TX_POLL_TIMEOUT_MS, TERRA_TX_POLL_TIMEOUT_MESSAGE)
}

async function recoverPostSignBroadcast(
  wallet: ConnectedWallet,
  txHash: string,
  signedSequence: bigint,
  recoveryDeadlineUnix: number,
  onPhaseChange?: TerraBroadcastOptions['onPhaseChange']
): Promise<string> {
  onPhaseChange?.('recovering', { txHash })
  const { txResponse } = await pollTxUntilRecoveryDeadline(wallet.rpc, txHash, recoveryDeadlineUnix)
  bumpWalletCachedSequence(wallet, signedSequence)

  if (txResponse.code !== 0) {
    const raw = txResponse.rawLog || txResponse.logs?.[0]?.log || `Transaction failed with code ${txResponse.code}`
    const human = tryHumanizeTerraTxMessage(raw)
    throw new Error(human ?? `Transaction failed: ${raw}`)
  }

  return txHash
}

async function broadcastSignedSplitPathAttempt(
  wallet: ConnectedWallet,
  unsignedTx: UnsignedTx,
  fee: ReturnType<typeof buildTerraClassicFee>,
  entries: TerraExecuteContractEntry[],
  onPhaseChange: TerraBroadcastOptions['onPhaseChange'] | undefined,
  signOptions?: SignTerraTxRawOptions
): Promise<string> {
  onPhaseChange?.('signing')

  const { txRaw, txHash, sequence } = await withTerraWalletSignLock(() =>
    signTerraTxRaw(wallet, unsignedTx, fee, signOptions)
  )

  onPhaseChange?.('broadcasting')

  const recoveryDeadlineUnix = resolveTerraTxRecoveryDeadlineUnix(entries)

  try {
    await withPromiseTimeout(
      RpcClient.broadcastTx(wallet.rpc, txRaw),
      TERRA_TX_BROADCAST_TIMEOUT_MS,
      TERRA_TX_BROADCAST_TIMEOUT_MESSAGE
    )
    bumpWalletCachedSequence(wallet, sequence)
  } catch (error: unknown) {
    if (!shouldRecoverPostSignBroadcast(error)) {
      throw error
    }
    onPhaseChange?.('confirming', { txHash })
    return recoverPostSignBroadcast(wallet, txHash, sequence, recoveryDeadlineUnix, onPhaseChange)
  }

  onPhaseChange?.('confirming', { txHash })

  let txResponse: Awaited<ReturnType<typeof pollTxWithTimeout>>['txResponse']
  try {
    ;({ txResponse } = await pollTxWithTimeout(wallet, txHash))
  } catch {
    return recoverPostSignBroadcast(wallet, txHash, sequence, recoveryDeadlineUnix, onPhaseChange)
  }

  if (txResponse.code !== 0) {
    const raw = txResponse.rawLog || txResponse.logs?.[0]?.log || `Transaction failed with code ${txResponse.code}`
    const human = tryHumanizeTerraTxMessage(raw)
    throw new Error(human ?? `Transaction failed: ${raw}`)
  }

  return txHash
}

/**
 * Split-path sign+broadcast with one automatic re-sign on Cosmos code-32 sequence mismatch ([GitLab #499](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/499)).
 * First attempt refreshes sequence from chain; retry uses the CheckTx-expected sequence when parseable.
 */
async function broadcastSignedSplitPath(
  wallet: ConnectedWallet,
  unsignedTx: UnsignedTx,
  fee: ReturnType<typeof buildTerraClassicFee>,
  entries: TerraExecuteContractEntry[],
  onPhaseChange?: TerraBroadcastOptions['onPhaseChange']
): Promise<string> {
  try {
    return await broadcastSignedSplitPathAttempt(wallet, unsignedTx, fee, entries, onPhaseChange, {
      useCachedSequence: false,
    })
  } catch (error: unknown) {
    const retry = planAccountSequenceRetry(wallet, error)
    if (!retry) throw error
    return broadcastSignedSplitPathAttempt(wallet, unsignedTx, fee, entries, onPhaseChange, retry)
  }
}

async function broadcastAtomicWalletPathAttempt(
  wallet: ConnectedWallet,
  unsignedTx: UnsignedTx,
  fee: ReturnType<typeof buildTerraClassicFee>,
  onPhaseChange: TerraBroadcastOptions['onPhaseChange'] | undefined,
  /** When true, keep code-32 expected sequence in cache; otherwise refresh from chain. */
  useCachedSequence: boolean
): Promise<string> {
  onPhaseChange?.('signing')
  // cosmes `broadcastTx` reads getAuthInfo(true); seed cache first (#499).
  await wallet.getAuthInfo(useCachedSequence)
  const txHash = await withTerraWalletSignLock(() => {
    onPhaseChange?.('broadcasting')
    return withPromiseTimeout(
      wallet.broadcastTx(unsignedTx, fee),
      TERRA_TX_BROADCAST_TIMEOUT_MS,
      TERRA_TX_BROADCAST_TIMEOUT_MESSAGE
    )
  })
  onPhaseChange?.('confirming', { txHash })

  const { txResponse } = await pollTxWithTimeout(wallet, txHash)
  if (txResponse.code !== 0) {
    const raw = txResponse.rawLog || txResponse.logs?.[0]?.log || `Transaction failed with code ${txResponse.code}`
    const human = tryHumanizeTerraTxMessage(raw)
    throw new Error(human ?? `Transaction failed: ${raw}`)
  }

  return txHash
}

/** Atomic WC/extension path: refresh sequence, then one code-32 re-broadcast ([GitLab #499](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/499)). */
async function broadcastAtomicWalletPath(
  wallet: ConnectedWallet,
  unsignedTx: UnsignedTx,
  fee: ReturnType<typeof buildTerraClassicFee>,
  onPhaseChange?: TerraBroadcastOptions['onPhaseChange']
): Promise<string> {
  try {
    return await broadcastAtomicWalletPathAttempt(wallet, unsignedTx, fee, onPhaseChange, false)
  } catch (error: unknown) {
    const retry = planAccountSequenceRetry(wallet, error)
    if (!retry) throw error
    return broadcastAtomicWalletPathAttempt(wallet, unsignedTx, fee, onPhaseChange, retry.useCachedSequence)
  }
}

/**
 * Canonical Terra Classic broadcast path: build msgs + fee, sign/broadcast, poll, map errors.
 * All `executeTerraContract*` entry points use this (GitLab #127).
 */
export async function broadcastTerraExecuteContracts(
  wallet: ConnectedWallet,
  walletAddress: string,
  entries: TerraExecuteContractEntry[],
  options?: TerraBroadcastOptions
): Promise<string> {
  const resolvedOptions = options ?? getTerraBroadcastScopeOptions()
  const onPhaseChange = resolvedOptions?.onPhaseChange
  if (entries.length === 0) {
    throw new Error('No messages to broadcast')
  }

  const msgs = entries.map(
    (entry) =>
      new MsgExecuteContract({
        sender: walletAddress,
        contract: entry.contract,
        msg: entry.msg,
        funds: entry.coins && entry.coins.length > 0 ? entry.coins : [],
      })
  )

  const unsignedTx: UnsignedTx = {
    msgs,
    memo: '',
  }

  const feeEstimate = estimateTerraClassicFeeForEntries(entries)
  const fee = buildTerraClassicFee(feeEstimate.gasLimit)

  if (wallet.id === WalletName.STATION && wallet.type === WalletType.EXTENSION) {
    await prepareStationExtensionForTerraClassicSign(wallet)
  }

  const useSplitPath = walletSupportsSplitSignBroadcast(wallet) && !isAtomicWalletConnectPost(wallet)

  try {
    if (useSplitPath) {
      return await broadcastSignedSplitPath(wallet, unsignedTx, fee, entries, onPhaseChange)
    }

    return await broadcastAtomicWalletPath(wallet, unsignedTx, fee, onPhaseChange)
  } catch (error: unknown) {
    console.error('Terra Classic transaction error:', error)
    throw handleBroadcastError(error)
  }
}
