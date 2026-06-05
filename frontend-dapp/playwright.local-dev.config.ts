import { defineConfig, devices } from '@playwright/test'
import base from './playwright.config'

/**
 * Run smoke/UI specs against an existing `make dev` server (default :5173).
 * No webServer — set PLAYWRIGHT_BASE_URL if the port differs.
 */
export default defineConfig({
  ...base,
  globalSetup: undefined,
  webServer: undefined,
  use: {
    ...base.use,
    baseURL: process.env.PLAYWRIGHT_BASE_URL ?? 'http://127.0.0.1:5173',
  },
  projects: [
    {
      name: 'e2e-smoke',
      testMatch: '**/verify-issue-295-ladder-rung-ui.spec.ts',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
})
