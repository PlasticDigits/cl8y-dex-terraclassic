import { execFileSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { join } from 'node:path'
import type { SwarmEnv } from './env.js'
import { lcdFetchJson } from './lcd.js'
import { getRepoRoot } from './paths.js'

export interface LocalnetValidation {
  repoRoot: string
  containerId: string
  chainId: string
}

interface NodeInfoResponse {
  default_node_info?: { network?: string }
}

export function getLocalterraContainerId(repoRoot: string): string {
  const compose = join(repoRoot, 'docker-compose.yml')
  if (!existsSync(compose)) {
    throw new Error(`docker-compose.yml not found at ${compose}`)
  }
  const out = execFileSync(
    'docker',
    ['compose', '-f', compose, 'ps', '-q', 'localterra'],
    { encoding: 'utf8', cwd: repoRoot }
  ).trim()
  if (!out) {
    throw new Error(
      'LocalTerra container is not running. Start it with: docker compose up -d localterra (from the repo root).'
    )
  }
  const first = out.split('\n')[0]?.trim()
  if (!first) {
    throw new Error('docker compose ps -q localterra returned empty output.')
  }
  return first
}

export async function assertLcdChainId(lcdBase: string, expected = 'localterra'): Promise<string> {
  const j = await lcdFetchJson<NodeInfoResponse>(
    lcdBase,
    '/cosmos/base/tendermint/v1beta1/node_info'
  )
  const id = j.default_node_info?.network
  if (id !== expected) {
    throw new Error(
      `Refusing to run: LCD reports chain_id/network "${id ?? 'unknown'}", expected "${expected}". ` +
        'The trading swarm is localnet-only.'
    )
  }
  return id ?? expected
}

export function assertRequiredEnv(e: SwarmEnv): void {
  if (!e.VITE_FACTORY_ADDRESS?.trim()) {
    throw new Error('VITE_FACTORY_ADDRESS is missing from .env.local (or env overrides).')
  }
  if (!e.VITE_ROUTER_ADDRESS?.trim()) {
    throw new Error('VITE_ROUTER_ADDRESS is missing from .env.local (or env overrides).')
  }
  if (!e.VITE_TERRA_LCD_URL?.trim()) {
    throw new Error('VITE_TERRA_LCD_URL is missing from .env.local (or env overrides).')
  }
  if (!e.VITE_TERRA_RPC_URL?.trim()) {
    throw new Error('VITE_TERRA_RPC_URL is missing from .env.local (or env overrides).')
  }
  if (e.VITE_NETWORK && e.VITE_NETWORK !== 'local') {
    throw new Error(
      `Refusing to run: VITE_NETWORK="${e.VITE_NETWORK}" is not "local". The swarm is for LocalTerra only.`
    )
  }
}

export async function validateLocalnet(env: SwarmEnv): Promise<LocalnetValidation> {
  const repoRoot = getRepoRoot()
  assertRequiredEnv(env)
  const lcd = env.VITE_TERRA_LCD_URL.replace(/\/$/, '')
  const chainId = await assertLcdChainId(lcd)
  const containerId = getLocalterraContainerId(repoRoot)
  return { repoRoot, containerId, chainId }
}
