import { execFileSync } from 'node:child_process'
import { lcdFetchJson, queryWasmSmart } from './lcd.js'
import {
  classifyCw20FundingKind,
  type Cw20FundingEnv,
  type Cw20FundingKind,
} from './fundingKind.js'
import type { LocalnetValidation } from './validateLocalnet.js'

const TERRAD_NODE = 'http://127.0.0.1:26657'
const CHAIN_ID = 'localterra'
/** Leave headroom on test1 for gas while funding bots (500k LUNC). */
const TEST1_ULUNA_GAS_RESERVE = 500_000_000_000n

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
  /** Defaults sized for LocalTerra genesis `test1` balances (11M LUNC / 100M USTC, GitLab #372); override with SWARM_* env if needed. */
  ulunaTopup: process.env.SWARM_ULUNA_TOPUP ?? '20000000000000',
  uusdTopup: process.env.SWARM_UUSD_TOPUP ?? '10000000000000',
  cw20MintTopup: process.env.SWARM_CW20_MINT_TOPUP ?? '100000000000000000',
  minCw20Balance: process.env.SWARM_MIN_CW20_BALANCE ?? '10000000000000',
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
    '--gas-prices',
    process.env.DEPLOY_GAS_PRICES ?? '28.325uluna',
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

function test1Address(v: LocalnetValidation): string {
  return execFileSync(
    'docker',
    ['exec', v.containerId, 'terrad', 'keys', 'show', 'test1', '-a', '--keyring-backend', 'test'],
    { encoding: 'utf8' }
  ).trim()
}

async function bankBalance(lcdBase: string, address: string, denom: string): Promise<bigint> {
  const body = await lcdFetchJson<{ balances?: Array<{ denom: string; amount: string }> }>(
    lcdBase,
    `/cosmos/bank/v1beta1/balances/${address}`
  )
  const coin = body.balances?.find((b) => b.denom === denom)
  try {
    return BigInt(coin?.amount ?? '0')
  } catch {
    return 0n
  }
}

async function topUpBankDenom(opts: {
  v: LocalnetValidation
  lcdBase: string
  faucetAddr: string
  botAddresses: string[]
  denom: string
  targetBalance: string
  sleepMs: number
}): Promise<void> {
  const target = BigInt(opts.targetBalance)
  const deficits: Array<{ addr: string; deficit: bigint }> = []
  for (const addr of opts.botAddresses) {
    const bal = await bankBalance(opts.lcdBase, addr, opts.denom)
    if (bal < target) deficits.push({ addr, deficit: target - bal })
  }
  if (deficits.length === 0) return

  let faucetBal = await bankBalance(opts.lcdBase, opts.faucetAddr, opts.denom)
  if (opts.denom === 'uluna' && faucetBal > TEST1_ULUNA_GAS_RESERVE) {
    faucetBal -= TEST1_ULUNA_GAS_RESERVE
  } else if (opts.denom === 'uluna') {
    faucetBal = 0n
  }

  const needs = deficits.map(({ addr, deficit }) => ({ addr, deficit }))
  let remaining = faucetBal
  while (remaining > 0n) {
    const activeCount = needs.reduce((n, entry) => n + (entry.deficit > 0n ? 1 : 0), 0)
    if (activeCount === 0) break

    let progress = false
    let slotsLeft = BigInt(activeCount)
    for (const entry of needs) {
      if (remaining <= 0n || entry.deficit <= 0n) continue
      const fairShare = remaining / slotsLeft
      const send = entry.deficit < fairShare ? entry.deficit : fairShare
      slotsLeft -= 1n
      if (send <= 0n) continue
      terradTx(opts.v, ['bank', 'send', 'test1', entry.addr, `${send}${opts.denom}`])
      remaining -= send
      entry.deficit -= send
      progress = true
      await pauseFunding(opts.sleepMs)
    }
    if (!progress) break
  }
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

async function resolveCw20FundingKind(
  lcdBase: string,
  token: string,
  env: Cw20FundingEnv
): Promise<Cw20FundingKind> {
  const pinned = classifyCw20FundingKind(token, env)
  if (pinned !== 'mint') return pinned
  try {
    const origin = await queryWasmSmart<{ launcher?: string | null }>(lcdBase, token, {
      get_launcher_origin: {},
    })
    return classifyCw20FundingKind(token, env, origin.launcher ?? null)
  } catch {
    return 'mint'
  }
}

export async function fundBotWallets(opts: {
  v: LocalnetValidation
  lcdBase: string
  botAddresses: string[]
  cw20Tokens: string[]
  funding: FundingOptions
  fundingEnv?: Cw20FundingEnv
}): Promise<void> {
  const { v, lcdBase, botAddresses, cw20Tokens, funding } = opts
  const fundingEnv: Cw20FundingEnv = opts.fundingEnv ?? { wrapAddresses: [] }
  const faucetAddr = test1Address(v)

  await topUpBankDenom({
    v,
    lcdBase,
    faucetAddr,
    botAddresses,
    denom: 'uluna',
    targetBalance: funding.ulunaTopup,
    sleepMs: funding.sleepMsBetweenFundingTx,
  })
  await topUpBankDenom({
    v,
    lcdBase,
    faucetAddr,
    botAddresses,
    denom: 'uusd',
    targetBalance: funding.uusdTopup,
    sleepMs: funding.sleepMsBetweenFundingTx,
  })

  const minB = BigInt(funding.minCw20Balance)

  for (const token of cw20Tokens) {
    const kind = await resolveCw20FundingKind(lcdBase, token, fundingEnv)
    if (kind === 'skip') continue
    for (const addr of botAddresses) {
      const bal = await cw20Balance(lcdBase, token, addr)
      if (bal >= minB) continue
      if (kind === 'transfer') {
        terradTx(v, [
          'wasm',
          'execute',
          token,
          JSON.stringify({ transfer: { recipient: addr, amount: funding.cw20MintTopup } }),
        ])
      } else {
        terradTx(v, [
          'wasm',
          'execute',
          token,
          JSON.stringify({ mint: { recipient: addr, amount: funding.cw20MintTopup } }),
        ])
      }
      await pauseFunding(funding.sleepMsBetweenFundingTx)
      if (funding.sleepMsBetweenMint > 0) {
        await new Promise((r) => setTimeout(r, funding.sleepMsBetweenMint))
      }
    }
  }
}
