#!/usr/bin/env node
import { parseArgs } from 'node:util'
import { loadViteEnv } from './env.js'
import { getRepoRoot } from './paths.js'
import { loadProfiles } from './profiles.js'
import { createSwarmWallets, resolveSwarmMnemonic } from './wallets.js'
import { startSwarm } from './swarmRunner.js'
import { validateLocalnet } from './validateLocalnet.js'

async function main(): Promise<void> {
  const { values } = parseArgs({
    options: {
      'dry-run': { type: 'boolean', default: false },
      stats: { type: 'boolean', default: false },
      'env-file': { type: 'string' },
      help: { type: 'boolean', short: 'h', default: false },
    },
    strict: true,
    allowPositionals: false,
  })

  if (values.help) {
    console.log(`localnet-trading-swarm — LocalTerra-only load-testing bots (GitLab #119)

Options:
  --dry-run     Validate env and print planned activity only (no funding, no txs).
  --stats       On exit (SIGINT/SIGTERM), print JSON with per-bot mean inter-tx gaps.
  --env-file    Optional path to a VITE_* env file (defaults to frontend-dapp/.env.local).
  -h, --help    Show this message.

Environment:
  SWARM_REPO_ROOT       Repo root (auto-detected if unset).
  SWARM_BOT_MNEMONIC    Optional 12-word English mnemonic; five accounts use indices 0–4.
                        If unset, a fresh mnemonic is generated (printed once on stderr).

See packages/localnet-trading-swarm/README.md.`)
    process.exit(0)
  }

  const repoRoot = getRepoRoot()
  const env = loadViteEnv(repoRoot, values['env-file'])
  const v = await validateLocalnet(env)
  const profiles = loadProfiles()

  const mnemonic = resolveSwarmMnemonic()
  if (!process.env.SWARM_BOT_MNEMONIC?.trim()) {
    console.error(
      '[localnet-trading-swarm] Generated ephemeral mnemonic (set SWARM_BOT_MNEMONIC to reproduce addresses):\n' +
        mnemonic +
        '\n'
    )
  }

  const wallets = createSwarmWallets({
    mnemonic,
    chainId: v.chainId,
    rpc: env.VITE_TERRA_RPC_URL.replace(/\/$/, ''),
    gasPriceUluna: env.VITE_GAS_PRICE_ULUNA ?? '28.325',
    count: 5,
  })

  console.log(
    JSON.stringify({
      ts: new Date().toISOString(),
      kind: 'swarm_start',
      chainId: v.chainId,
      bots: wallets.map((w, i) => ({ bot: i, address: w.address })),
      dryRun: !!values['dry-run'],
    })
  )

  const runner = await startSwarm({
    v,
    env,
    wallets,
    profiles,
    runnerOpts: {
      dryRun: !!values['dry-run'],
      collectStats: !!values.stats,
      meanSeconds: profiles.meanInterTxSeconds,
    },
  })

  const shutdown = (signal: string) => {
    console.error(`[localnet-trading-swarm] ${signal}, stopping…`)
    runner.stop()
    runner.printStats()
    process.exit(0)
  }
  process.on('SIGINT', () => shutdown('SIGINT'))
  process.on('SIGTERM', () => shutdown('SIGTERM'))

  await runner.waitUntilStopped()
}

main().catch((e) => {
  console.error(e instanceof Error ? e.message : e)
  process.exit(1)
})
