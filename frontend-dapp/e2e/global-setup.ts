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
function applyViteLcdFromEnvLocal(envLocalPath: string): void {
  const raw = fs.readFileSync(envLocalPath, 'utf8')
  for (const line of raw.split('\n')) {
    const m = line.match(/^VITE_TERRA_LCD_URL=(.+)$/)
    if (m) {
      process.env.VITE_TERRA_LCD_URL = m[1].trim().replace(/^["']|["']$/g, '')
      break
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
  if (process.env.REQUIRE_LOCALTERRA === '0') {
    return
  }

  const repoRoot = repoRootFromE2e()
  const envLocal = path.join(repoRoot, 'frontend-dapp', '.env.local')
  if (!fs.existsSync(envLocal)) {
    throw new Error(
      'E2E globalSetup: frontend-dapp/.env.local is missing. Run `bash scripts/deploy-dex-local.sh` with LocalTerra up, then re-run Playwright.'
    )
  }

  applyViteLcdFromEnvLocal(envLocal)
  const base = lcdBaseUrlForSetup()
  await waitForLcd(base, 120_000)

  const script = path.join(repoRoot, 'scripts', 'e2e-provision-dev-wallet.sh')
  execFileSync('bash', [script], {
    stdio: 'inherit',
    env: { ...process.env, REPO_ROOT: repoRoot },
    cwd: repoRoot,
  })
}
