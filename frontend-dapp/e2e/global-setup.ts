import { execFileSync } from 'node:child_process'
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

async function waitForLcd(base: string, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs
  let lastErr: string | undefined
  while (Date.now() < deadline) {
    try {
      const res = await fetch(`${base}/cosmos/base/tendermint/v1beta1/node_info`, {
        signal: AbortSignal.timeout(10_000),
      })
      if (res.ok) return
      lastErr = `HTTP ${res.status}`
    } catch (e) {
      lastErr = e instanceof Error ? e.message : String(e)
    }
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
  const base = lcdBaseUrlForSetup()
  await waitForLcd(base, 120_000)

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
