import { execFileSync } from 'node:child_process'
import { queryWasmSmart } from './lcd.js'
import type { LocalnetValidation } from './validateLocalnet.js'

const TERRAD_NODE = 'http://127.0.0.1:26657'
const CHAIN_ID = 'localterra'

export interface FundingOptions {
  ulunaTopup: string
  uusdTopup: string
  cw20MintTopup: string
  minCw20Balance: string
  /** Pause after each `terrad tx` so `test1` account sequence advances before the next broadcast (`sync` returns before inclusion). */
  sleepMsBetweenFundingTx: number
  sleepMsBetweenMint: number
}

export const defaultFundingOptions = (): FundingOptions => ({
  /** Defaults sized for typical LocalTerra genesis `test1` balances (~1e14 uluna / ~1e13 uusd); override with SWARM_* env if needed. */
  ulunaTopup: process.env.SWARM_ULUNA_TOPUP ?? '2000000000000',
  uusdTopup: process.env.SWARM_UUSD_TOPUP ?? '1000000000000',
  cw20MintTopup: process.env.SWARM_CW20_MINT_TOPUP ?? '10000000000000000',
  minCw20Balance: process.env.SWARM_MIN_CW20_BALANCE ?? '1000000000000',
  sleepMsBetweenFundingTx: Number(process.env.SWARM_FUNDING_TX_SLEEP_MS ?? '2000'),
  sleepMsBetweenMint: Number(process.env.SWARM_MINT_SLEEP_MS ?? '500'),
})

function terradTx(v: LocalnetValidation, args: string[]): void {
  const full = [
    'exec',
    v.containerId,
    'terrad',
    'tx',
    ...args,
    '--from',
    'test1',
    '--keyring-backend',
    'test',
    '--chain-id',
    CHAIN_ID,
    '--gas',
    'auto',
    '--gas-adjustment',
    '1.3',
    '--fees',
    '500000000uluna',
    '--node',
    TERRAD_NODE,
    '--broadcast-mode',
    'sync',
    '-y',
    '--output',
    'json',
  ]
  execFileSync('docker', full, { stdio: ['ignore', 'pipe', 'inherit'] })
}

async function pauseFunding(ms: number): Promise<void> {
  if (ms > 0) await new Promise((r) => setTimeout(r, ms))
}

async function cw20Balance(
  lcdBase: string,
  token: string,
  holder: string
): Promise<bigint> {
  const raw = await queryWasmSmart<{ balance: string }>(lcdBase, token, {
    balance: { address: holder },
  })
  try {
    return BigInt(raw.balance ?? '0')
  } catch {
    return 0n
  }
}

export async function fundBotWallets(opts: {
  v: LocalnetValidation
  lcdBase: string
  botAddresses: string[]
  cw20Tokens: string[]
  funding: FundingOptions
}): Promise<void> {
  const { v, lcdBase, botAddresses, cw20Tokens, funding } = opts

  for (const addr of botAddresses) {
    terradTx(v, ['bank', 'send', 'test1', addr, `${funding.ulunaTopup}uluna`])
    await pauseFunding(funding.sleepMsBetweenFundingTx)
    terradTx(v, ['bank', 'send', 'test1', addr, `${funding.uusdTopup}uusd`])
    await pauseFunding(funding.sleepMsBetweenFundingTx)
  }

  const minB = BigInt(funding.minCw20Balance)

  for (const token of cw20Tokens) {
    for (const addr of botAddresses) {
      const bal = await cw20Balance(lcdBase, token, addr)
      if (bal >= minB) continue
      terradTx(v, [
        'wasm',
        'execute',
        token,
        JSON.stringify({ mint: { recipient: addr, amount: funding.cw20MintTopup } }),
      ])
      await pauseFunding(funding.sleepMsBetweenFundingTx)
      if (funding.sleepMsBetweenMint > 0) {
        await new Promise((r) => setTimeout(r, funding.sleepMsBetweenMint))
      }
    }
  }
}
