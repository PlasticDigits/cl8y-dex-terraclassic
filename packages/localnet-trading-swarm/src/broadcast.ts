import { MsgExecuteContract } from '@goblinhunt/cosmes/client'
import type { MnemonicWallet, UnsignedTx } from '@goblinhunt/cosmes/wallet'
import { estimateTerraClassicFee, getGasLimitForExecuteMsg } from './gas.js'

export async function executeWasm(
  wallet: MnemonicWallet,
  contract: string,
  msg: Record<string, unknown>,
  coins: Array<{ denom: string; amount: string }> = [],
  gasPriceUluna: string
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
  const txHash = await wallet.broadcastTx(unsignedTx, fee)
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

export async function executeWasmMulti(
  wallet: MnemonicWallet,
  steps: Array<{
    contract: string
    msg: Record<string, unknown>
    coins?: Array<{ denom: string; amount: string }>
  }>,
  gasPriceUluna: string
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
  const txHash = await wallet.broadcastTx(unsignedTx, fee)
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
