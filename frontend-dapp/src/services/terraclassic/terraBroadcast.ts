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
import { withTerraWalletSignLock } from './terraWalletSignLock'

export type TerraExecuteContractEntry = {
  contract: string
  msg: Record<string, unknown>
  coins?: Array<{ denom: string; amount: string }>
}

function handleBroadcastError(error: unknown): Error {
  if (error instanceof Error) {
    const errorMessage = error.message

    if (errorMessage === TERRA_TX_BROADCAST_TIMEOUT_MESSAGE || errorMessage === TERRA_TX_POLL_TIMEOUT_MESSAGE) {
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
  entries: TerraExecuteContractEntry[]
): Promise<string> {
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

  try {
    const txHash = await withTerraWalletSignLock(() =>
      withPromiseTimeout(
        wallet.broadcastTx(unsignedTx, fee),
        TERRA_TX_BROADCAST_TIMEOUT_MS,
        TERRA_TX_BROADCAST_TIMEOUT_MESSAGE
      )
    )
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
  }
}
