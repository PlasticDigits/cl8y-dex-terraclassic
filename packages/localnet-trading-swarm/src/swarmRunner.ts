import type { MnemonicWallet } from '@goblinhunt/cosmes/wallet'
import type { ActionContext } from './actions.js'
import { runAction } from './actions.js'
import { fundBotWallets, defaultFundingOptions } from './funding.js'
import { uniqueCw20TokenAddresses, fetchAllPairs } from './factoryTokens.js'
import { pickActionKind, type ProfileConfig, type ProfilesFile } from './profiles.js'
import { createTxQueue, sampleInterTxDelaySeconds, GapAccumulator } from './scheduler.js'
import type { LocalnetValidation } from './validateLocalnet.js'
import type { SwarmEnv } from './env.js'

export interface SwarmRunnerOptions {
  dryRun: boolean
  collectStats: boolean
  meanSeconds: number
}

export interface SwarmRunner {
  stop: () => void
  waitUntilStopped: () => Promise<void>
  printStats: () => void
}

export async function startSwarm(opts: {
  v: LocalnetValidation
  env: SwarmEnv
  wallets: MnemonicWallet[]
  profiles: ProfilesFile
  runnerOpts: SwarmRunnerOptions
}): Promise<SwarmRunner> {
  const { v, env, wallets, profiles } = opts
  const lcd = env.VITE_TERRA_LCD_URL.replace(/\/$/, '')
  const factory = env.VITE_FACTORY_ADDRESS
  const router = env.VITE_ROUTER_ADDRESS
  const gasPrice = env.VITE_GAS_PRICE_ULUNA ?? '28.325'

  const mean = opts.runnerOpts.meanSeconds || profiles.meanInterTxSeconds || 20

  const pairs = await fetchAllPairs(lcd, factory)
  const cw20s = uniqueCw20TokenAddresses(pairs)
  const botAddresses = wallets.map((w) => w.address)

  if (!opts.runnerOpts.dryRun) {
    await fundBotWallets({
      v,
      lcdBase: lcd,
      botAddresses,
      cw20Tokens: cw20s,
      funding: defaultFundingOptions(),
    })
  }

  const ctx: ActionContext = {
    lcdBase: lcd,
    router,
    pairs,
    gasPriceUluna: gasPrice,
    dryRun: opts.runnerOpts.dryRun,
  }

  const queues = wallets.map(() => createTxQueue())
  const gapAcc = new GapAccumulator(wallets.length)
  const lastActionAt: (number | null)[] = wallets.map(() => null)
  const perBotTimer: (NodeJS.Timeout | null)[] = wallets.map(() => null)
  let stopped = false
  let stopResolve: (() => void) | null = null
  const stopPromise = new Promise<void>((r) => {
    stopResolve = r
  })

  const profileList = profiles.profiles as ProfileConfig[]

  function logLine(obj: Record<string, unknown>): void {
    console.log(JSON.stringify({ ts: new Date().toISOString(), ...obj }))
  }

  function scheduleBot(botIndex: number): void {
    if (stopped) return
    const prev = perBotTimer[botIndex]
    if (prev) clearTimeout(prev)
    const delayMs = sampleInterTxDelaySeconds(mean) * 1000
    perBotTimer[botIndex] = setTimeout(() => {
      perBotTimer[botIndex] = null
      void runBotTurn(botIndex).finally(() => {
        if (!stopped) scheduleBot(botIndex)
      })
    }, delayMs)
  }

  async function runBotTurn(botIndex: number): Promise<void> {
    if (stopped) return
    const wallet = wallets[botIndex]!
    const queue = queues[botIndex]!
    const profile = profileList[botIndex] ?? profileList[0]!
    const roll = Math.random()
    const kind = pickActionKind(profile, roll)

    await queue(async () => {
      try {
        const res = await runAction(kind, wallet, ctx)
        const now = Date.now() / 1000
        const prev = lastActionAt[botIndex]
        lastActionAt[botIndex] = now
        if (opts.runnerOpts.collectStats && prev !== null) {
          gapAcc.push(botIndex, now - prev)
        }
        logLine({
          profile: profile.id,
          bot: botIndex,
          action: res.action,
          txHash: res.txHash ?? null,
          note: res.note ?? null,
          dryRun: res.dryRun ?? false,
        })
      } catch (err) {
        logLine({
          profile: profile.id,
          bot: botIndex,
          action: kind,
          error: err instanceof Error ? err.message : String(err),
        })
      }
    })
  }

  for (let i = 0; i < wallets.length; i++) {
    scheduleBot(i)
  }

  return {
    stop: () => {
      if (stopped) return
      stopped = true
      for (let i = 0; i < perBotTimer.length; i++) {
        const t = perBotTimer[i]
        if (t) clearTimeout(t)
        perBotTimer[i] = null
      }
      stopResolve?.()
    },
    waitUntilStopped: () => stopPromise,
    printStats: () => {
      if (!opts.runnerOpts.collectStats) return
      const rows = gapAcc.summary(mean)
      console.log(
        JSON.stringify({
          ts: new Date().toISOString(),
          kind: 'swarm_stats',
          meanTargetSec: mean,
          perBot: rows,
        })
      )
    },
  }
}
