import { defineConfig, devices } from '@playwright/test'
import { getLocalTerraTestMnemonic } from './e2e/localterra-mnemonic'

const devMnemonic = getLocalTerraTestMnemonic()

/** UI-only local dev: skip chain global setup and allow documented test.skip (GitLab #201). */
const chainOptional = process.env.PLAYWRIGHT_SKIP_CHAIN === '1' || process.env.REQUIRE_LOCALTERRA === '0'

const txSpecGlobs = ['**/*-tx.spec.ts', '**/hybrid-swap.spec.ts', '**/wrap-pool.spec.ts', '**/wrap-swap.spec.ts']
/** Market-data-down specs; require E2E_INDEXER_OUTAGE=1 and stopped indexer (GitLab #219). */
const indexerOutageGlobs = ['**/*-indexer-outage.spec.ts']

/** On-chain tx specs share one LocalTerra account — parallel workers cause sequence mismatch (#201). */
function playwrightProjectArg(): string | undefined {
  const idx = process.argv.indexOf('--project')
  if (idx >= 0 && process.argv[idx + 1]) return process.argv[idx + 1]
  const eq = process.argv.find((a) => a.startsWith('--project='))
  return eq?.slice('--project='.length)
}

const e2eTxProjectOnly = playwrightProjectArg() === 'e2e-tx'

export default defineConfig({
  testDir: './e2e',
  globalSetup: chainOptional ? undefined : './e2e/global-setup.ts',
  fullyParallel: !e2eTxProjectOnly,
  workers: e2eTxProjectOnly ? 1 : 5,
  timeout: 120_000,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI ? 'github' : 'html',
  use: {
    baseURL: process.env.PLAYWRIGHT_BASE_URL ?? 'http://localhost:3000',
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
    viewport: { width: 1280, height: 720 },
  },
  webServer: {
    command: 'VITE_NETWORK=local VITE_DEV_MODE=true npm run dev',
    port: 3000,
    reuseExistingServer: true,
    timeout: 60_000,
    env: {
      ...process.env,
      VITE_DEV_MNEMONIC: devMnemonic,
      /** Suppress blocking first-visit risk modal during Playwright (GitLab #138). */
      VITE_PLAYWRIGHT_E2E: 'true',
      /** Fast indexer transport failure for outage project (GitLab #219). */
      VITE_E2E_INDEXER_OUTAGE: process.env.E2E_INDEXER_OUTAGE ?? '',
    },
  },
  projects: [
    {
      name: 'e2e-smoke',
      testIgnore: [...txSpecGlobs, ...indexerOutageGlobs],
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'e2e-tx',
      testMatch: txSpecGlobs,
      fullyParallel: false,
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'e2e-indexer-outage',
      testMatch: indexerOutageGlobs,
      use: { ...devices['Desktop Chrome'] },
    },
  ],
})
