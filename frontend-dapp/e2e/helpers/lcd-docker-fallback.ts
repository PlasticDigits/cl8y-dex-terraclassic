import { execFileSync } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import type { APIRequestContext } from '@playwright/test'

import { lcdBaseUrl } from './chain'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const REPO_ROOT = path.join(__dirname, '..', '..', '..')
const COMPOSE_FILE = path.join(REPO_ROOT, 'docker-compose.yml')
const NODE_INFO = '/cosmos/base/tendermint/v1beta1/node_info'

/** Playwright / Vite should use the loopback proxy when set (see scripts/e2e-lcd-proxy.mjs). */
export function effectiveLcdBaseUrl(): string {
  const proxy = process.env.E2E_LCD_PROXY_URL?.replace(/\/$/, '')
  if (proxy) return proxy
  return lcdBaseUrl()
}

function localterraContainerId(): string {
  return execFileSync('docker', ['compose', '-f', COMPOSE_FILE, 'ps', '-q', 'localterra'], {
    encoding: 'utf8',
    cwd: REPO_ROOT,
  }).trim()
}

export function lcdHealthyViaDockerExec(): boolean {
  try {
    const cid = localterraContainerId()
    if (!cid) return false
    execFileSync(
      'docker',
      ['exec', cid, 'curl', '-sf', '--connect-timeout', '2', '--max-time', '8', `http://127.0.0.1:1317${NODE_INFO}`],
      { stdio: 'pipe', cwd: REPO_ROOT }
    )
    return true
  } catch {
    return false
  }
}

export async function lcdRequestGet(
  request: APIRequestContext,
  urlPath: string,
  opts?: { timeout?: number; failOnStatusCode?: boolean }
): Promise<{ ok: boolean; status: number; json: () => Promise<unknown> }> {
  const base = effectiveLcdBaseUrl()
  const timeout = opts?.timeout ?? 20_000
  try {
    const res = await request.get(`${base}${urlPath}`, {
      timeout,
      failOnStatusCode: opts?.failOnStatusCode ?? false,
    })
    return {
      ok: res.ok(),
      status: res.status(),
      json: () => res.json(),
    }
  } catch {
    const cid = localterraContainerId()
    if (!cid) throw new Error(`LCD ${base} unreachable and localterra container is down`)
    const inner = `http://127.0.0.1:1317${urlPath}`
    const raw = execFileSync(
      'docker',
      ['exec', cid, 'curl', '-sf', '-w', '\n%{http_code}', '--max-time', '60', inner],
      { encoding: 'utf8', maxBuffer: 32 * 1024 * 1024, cwd: REPO_ROOT }
    )
    const nl = raw.lastIndexOf('\n')
    const body = raw.slice(0, nl)
    const status = Number(raw.slice(nl + 1))
    const parsed = JSON.parse(body) as unknown
    return {
      ok: status >= 200 && status < 300,
      status,
      json: async () => parsed,
    }
  }
}
