import { MsgExecuteContract } from '@goblinhunt/cosmes/client'
import type { MnemonicWallet, UnsignedTx } from '@goblinhunt/cosmes/wallet'
import { estimateTerraClassicFee, getGasLimitForExecuteMsg } from './gas.js'

export type SwarmBroadcastPhase = 'signing' | 'broadcasting' | 'confirming'

export type SwarmBroadcastOptions = {
  onPhaseChange?: (phase: SwarmBroadcastPhase, ctx?: { txHash?: string }) => void
}

async function broadcastAndPoll(
  wallet: MnemonicWallet,
  unsignedTx: UnsignedTx,
  fee: ReturnType<typeof estimateTerraClassicFee>,
  options?: SwarmBroadcastOptions
): Promise<string> {
  const onPhaseChange = options?.onPhaseChange
  onPhaseChange?.('signing')
  onPhaseChange?.('broadcasting')
  const txHash = await wallet.broadcastTx(unsignedTx, fee)
  onPhaseChange?.('confirming', { txHash })
  const { txResponse } = await wallet.pollTx(txHash)
  if (txResponse.code !== 0) {
    const err =
      txResponse.rawLog ||
      txResponse.logs?.[0]?.log ||
      `code ${txResponse.code}`
    throw new Error(err)
  }
  return txHash
}

export async function executeWasm(
  wallet: MnemonicWallet,
  contract: string,
  msg: Record<string, unknown>,
  coins: Array<{ denom: string; amount: string }> = [],
  gasPriceUluna: string,
  options?: SwarmBroadcastOptions
): Promise<string> {
  const m = new MsgExecuteContract({
    sender: wallet.address,
    contract,
    msg,
    funds: coins,
  })
  const unsignedTx: UnsignedTx = { msgs: [m], memo: 'localnet-swarm' }
  const gas = getGasLimitForExecuteMsg(msg)
  const fee = estimateTerraClassicFee(gas, gasPriceUluna)
  return broadcastAndPoll(wallet, unsignedTx, fee, options)
}

export async function executeWasmMulti(
  wallet: MnemonicWallet,
  steps: Array<{
    contract: string
    msg: Record<string, unknown>
    coins?: Array<{ denom: string; amount: string }>
  }>,
  gasPriceUluna: string,
  options?: SwarmBroadcastOptions
): Promise<string> {
  const msgs = steps.map(
    (s) =>
      new MsgExecuteContract({
        sender: wallet.address,
        contract: s.contract,
        msg: s.msg,
        funds: s.coins ?? [],
      })
  )
  const unsignedTx: UnsignedTx = { msgs, memo: 'localnet-swarm' }
  const gas = steps.reduce((sum, s) => sum + getGasLimitForExecuteMsg(s.msg), 0)
  const fee = estimateTerraClassicFee(gas, gasPriceUluna)
  return broadcastAndPoll(wallet, unsignedTx, fee, options)
}
