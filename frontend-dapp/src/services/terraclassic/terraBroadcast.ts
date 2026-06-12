import { MsgExecuteContract } from '@goblinhunt/cosmes/client'
import type { ConnectedWallet } from '@goblinhunt/cosmes/wallet'
import type { UnsignedTx } from '@goblinhunt/cosmes/wallet'
import { WalletName, WalletType } from '@goblinhunt/cosmes/wallet'
import { prepareStationExtensionForTerraClassicSign } from '@/services/terraclassic/stationExtensionConfig'
import { estimateTerraClassicFeeForEntries } from '@/services/terraclassic/terraClassicFeeEstimate'
import { EXTENSION_SIGNED_FEE_UNDERSHOOT_PREFIX } from '@/utils/extensionSignedFeeGuard'
import { tryHumanizeTerraTxMessage } from '@/utils/humanizeTerraTxError'
import {
  TERRA_TX_BROADCAST_TIMEOUT_MESSAGE,
  TERRA_TX_BROADCAST_TIMEOUT_MS,
  TERRA_TX_POLL_TIMEOUT_MESSAGE,
  TERRA_TX_POLL_TIMEOUT_MS,
} from '@/utils/terraTxTimeout'
import { withPromiseTimeout } from '@/utils/withPromiseTimeout'
import { buildTerraClassicFee } from './terraGas'
import { getTerraBroadcastScopeOptions } from './terraBroadcastScope'
import { terraRecoveryPollDeadlineUnix } from './terraMsgDeadline'
import { pollTerraTxRecovery, TERRA_TX_RECOVERY_EXPIRED_MESSAGE } from './terraTxRecoveryPoll'
import { installSignedTxHashCapture } from './terraWalletSignTxRaw'
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
    /insufficient fees?|minimum fee|auth info validation|signatures validation/i.test(msg) ||
    /tx already exists in cache/i.test(msg)
  )
}

/**
 * After sign, ambiguous RPC failures may still have submitted the tx — poll until deadline.
 * Definite CheckTx rejections skip recovery so the user gets the original error promptly.
 */
function isPostSignBroadcastFailure(error: unknown, signedTxHash: string | null): boolean {
  if (!signedTxHash) return false
  if (isDefiniteBroadcastRejection(error)) return false
  return true
}

function handleBroadcastError(error: unknown): Error {
  if (error instanceof Error) {
    const errorMessage = error.message

    if (
      errorMessage === TERRA_TX_BROADCAST_TIMEOUT_MESSAGE ||
      errorMessage === TERRA_TX_POLL_TIMEOUT_MESSAGE ||
      errorMessage === TERRA_TX_RECOVERY_EXPIRED_MESSAGE
    ) {
      return error
    }

    if (errorMessage.includes(EXTENSION_SIGNED_FEE_UNDERSHOOT_PREFIX)) {
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

  const capture = installSignedTxHashCapture()
  try {
    onPhaseChange?.('signing')
    let txHash: string
    try {
      txHash = await withTerraWalletSignLock(() => {
        onPhaseChange?.('broadcasting')
        return withPromiseTimeout(
          wallet.broadcastTx(unsignedTx, fee),
          TERRA_TX_BROADCAST_TIMEOUT_MS,
          TERRA_TX_BROADCAST_TIMEOUT_MESSAGE
        )
      })
    } catch (broadcastError: unknown) {
      const signedHash = capture.signedTxHash
      if (isPostSignBroadcastFailure(broadcastError, signedHash)) {
        onPhaseChange?.('recovering', { txHash: signedHash! })
        await pollTerraTxRecovery(wallet, signedHash!, terraRecoveryPollDeadlineUnix(entries))
        return signedHash!
      }
      throw broadcastError
    }

    onPhaseChange?.('confirming', { txHash })
    const { txResponse } = await withPromiseTimeout(
      wallet.pollTx(txHash),
      TERRA_TX_POLL_TIMEOUT_MS,
      TERRA_TX_POLL_TIMEOUT_MESSAGE
    )

    if (txResponse.code !== 0) {
      const raw = txResponse.rawLog || txResponse.logs?.[0]?.log || `Transaction failed with code ${txResponse.code}`
      const human = tryHumanizeTerraTxMessage(raw)
      throw new Error(human ?? `Transaction failed: ${raw}`)
    }

    return txHash
  } catch (error: unknown) {
    console.error('Terra Classic transaction error:', error)
    throw handleBroadcastError(error)
  } finally {
    capture.restore()
  }
}
