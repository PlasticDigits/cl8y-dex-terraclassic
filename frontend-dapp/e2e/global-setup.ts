import { execFileSync, spawn } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

const DEFAULT_LCD = 'http://localhost:1317'

function lcdBaseUrlForSetup(): string {
  const u = process.env.VITE_TERRA_LCD_URL || process.env.E2E_LCD_URL || DEFAULT_LCD
  return u.replace(/\/$/, '')
}

function repoRootFromE2e(): string {
  // frontend-dapp/e2e -> repo root
  return path.join(__dirname, '..', '..')
}

/** Playwright does not load Vite `.env.local`; read LCD URL for setup wait + provision. */
function applyViteEnvFromEnvLocal(envLocalPath: string): void {
  const raw = fs.readFileSync(envLocalPath, 'utf8')
  for (const line of raw.split('\n')) {
    const lcd = line.match(/^VITE_TERRA_LCD_URL=(.+)$/)
    if (lcd) {
      process.env.VITE_TERRA_LCD_URL = lcd[1].trim().replace(/^["']|["']$/g, '')
    }
    const idx = line.match(/^VITE_INDEXER_URL=(.+)$/)
    if (idx) {
      process.env.VITE_INDEXER_URL = idx[1].trim().replace(/^["']|["']$/g, '')
    }
  }
}

const LCD_NODE_INFO_PATH = '/cosmos/base/tendermint/v1beta1/node_info'

/** In-container LCD when host :1317 hangs (docker userland-proxy; GitLab #292 / #206). */
function lcdHealthyViaDockerExec(repoRoot: string): boolean {
  const composeFile = path.join(repoRoot, 'docker-compose.yml')
  try {
    const cid = execFileSync('docker', ['compose', '-f', composeFile, 'ps', '-q', 'localterra'], {
      encoding: 'utf8',
      cwd: repoRoot,
    }).trim()
    if (!cid) return false
    execFileSync(
      'docker',
      [
        'exec',
        cid,
        'curl',
        '-sf',
        '--connect-timeout',
        '2',
        '--max-time',
        '8',
        `http://127.0.0.1:1317${LCD_NODE_INFO_PATH}`,
      ],
      { stdio: 'pipe', cwd: repoRoot }
    )
    return true
  } catch {
    return false
  }
}

/** When host :1317 hangs, start scripts/e2e-lcd-proxy.mjs for Vite + Playwright (LT9). */
async function ensureE2eLcdProxy(repoRoot: string, hostBase: string): Promise<void> {
  try {
    const res = await fetch(`${hostBase}${LCD_NODE_INFO_PATH}`, { signal: AbortSignal.timeout(5_000) })
    if (res.ok) return
  } catch {
    /* fall through */
  }
  if (!lcdHealthyViaDockerExec(repoRoot)) return

  const proxyPort = process.env.E2E_LCD_PROXY_PORT || '13170'
  const proxyUrl = `http://127.0.0.1:${proxyPort}`
  const proxyScript = path.join(repoRoot, 'scripts', 'e2e-lcd-proxy.mjs')
  spawn(process.execPath, [proxyScript], {
    detached: true,
    stdio: 'ignore',
    env: { ...process.env, E2E_LCD_PROXY_PORT: proxyPort },
    cwd: repoRoot,
  }).unref()

  const deadline = Date.now() + 30_000
  while (Date.now() < deadline) {
    try {
      const res = await fetch(`${proxyUrl}${LCD_NODE_INFO_PATH}`, { signal: AbortSignal.timeout(3_000) })
      if (res.ok) {
        process.env.E2E_LCD_PROXY_URL = proxyUrl
        process.env.VITE_TERRA_LCD_URL = proxyUrl
        process.env.E2E_LCD_URL = proxyUrl
        console.log(`E2E globalSetup: using LCD proxy ${proxyUrl} (host ${hostBase} unreachable)`)
        return
      }
    } catch {
      /* retry */
    }
    await new Promise((r) => setTimeout(r, 500))
  }
  throw new Error(`E2E globalSetup: LCD proxy did not become ready on ${proxyUrl}`)
}

async function waitForLcd(base: string, timeoutMs: number, repoRoot: string): Promise<void> {
  const deadline = Date.now() + timeoutMs
  let lastErr: string | undefined
  while (Date.now() < deadline) {
    try {
      const res = await fetch(`${base}${LCD_NODE_INFO_PATH}`, {
        signal: AbortSignal.timeout(10_000),
      })
      if (res.ok) return
      lastErr = `HTTP ${res.status}`
    } catch (e) {
      lastErr = e instanceof Error ? e.message : String(e)
    }
    if (lcdHealthyViaDockerExec(repoRoot)) return
    await new Promise((r) => setTimeout(r, 2000))
  }
  throw new Error(`E2E globalSetup: LCD ${base} not reachable after ${timeoutMs}ms (${lastErr ?? 'unknown'})`)
}

export default async function globalSetup(): Promise<void> {
  if (process.env.PLAYWRIGHT_SKIP_CHAIN === '1' || process.env.REQUIRE_LOCALTERRA === '0') {
    return
  }

  const repoRoot = repoRootFromE2e()
  const envLocal = path.join(repoRoot, 'frontend-dapp', '.env.local')
  if (!fs.existsSync(envLocal)) {
    throw new Error(
      'E2E globalSetup: frontend-dapp/.env.local is missing. Run `bash scripts/deploy-dex-local.sh` with LocalTerra up, then re-run Playwright.'
    )
  }

  applyViteEnvFromEnvLocal(envLocal)

  const stampPath = path.join(repoRoot, '.qa-deploy-stamp')
  if (fs.existsSync(stampPath)) {
    const pairLine = fs.readFileSync(stampPath, 'utf8').match(/^pair_address=(.+)$/m)
    if (pairLine?.[1]) {
      process.env.E2E_TRADE_PAIR = pairLine[1].trim()
    }
  }

  const base = lcdBaseUrlForSetup()
  await waitForLcd(base, 120_000, repoRoot)
  await ensureE2eLcdProxy(repoRoot, base)

  // Indexer-outage specs are read-only UI; skip mint/seed (avoids sequence races with local bots).
  if (process.env.E2E_INDEXER_OUTAGE === '1') {
    // Playwright uses OUTAGE_E2E_INDEXER_URL (unbound port); optional stop of :3001 for shared hosts.
    if (process.env.VITE_INDEXER_URL?.includes(':3001')) {
      const stopIndexer = path.join(repoRoot, 'scripts', 'lib', 'indexer-stop-for-outage-e2e.sh')
      execFileSync('bash', [stopIndexer], {
        stdio: 'inherit',
        env: { ...process.env, REPO_ROOT: repoRoot },
        cwd: repoRoot,
      })
    }
    return
  }

  const provision = path.join(repoRoot, 'scripts', 'e2e-provision-dev-wallet.sh')
  execFileSync('bash', [provision], {
    stdio: 'inherit',
    env: { ...process.env, REPO_ROOT: repoRoot },
    cwd: repoRoot,
  })

  const hybridSeed = path.join(repoRoot, 'scripts', 'e2e-seed-hybrid-book.sh')
  execFileSync('bash', [hybridSeed], {
    stdio: 'inherit',
    env: { ...process.env, REPO_ROOT: repoRoot },
    cwd: repoRoot,
  })

  const wrapPairsSeed = path.join(repoRoot, 'scripts', 'e2e-seed-wrap-pairs.sh')
  execFileSync('bash', [wrapPairsSeed], {
    stdio: 'inherit',
    env: { ...process.env, REPO_ROOT: repoRoot },
    cwd: repoRoot,
  })
}
