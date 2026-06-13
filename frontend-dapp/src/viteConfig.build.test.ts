/** @vitest-environment node */
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { loadConfigFromFile } from 'vite'
import { describe, expect, it } from 'vitest'

const viteConfigPath = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', 'vite.config.ts')

function withEnv<T>(vars: Record<string, string | undefined>, fn: () => Promise<T>): Promise<T> {
  const prev: Record<string, string | undefined> = {}
  for (const [key, value] of Object.entries(vars)) {
    prev[key] = process.env[key]
    if (value === undefined) {
      delete process.env[key]
    } else {
      process.env[key] = value
    }
  }
  return fn().finally(() => {
    for (const [key, value] of Object.entries(prev)) {
      if (value === undefined) {
        delete process.env[key]
      } else {
        process.env[key] = value
      }
    }
  })
}

describe('vite.config production source maps', () => {
  it('disables build.sourcemap for production mode (GitLab #117)', async () => {
    await withEnv({ VITE_WC_PROJECT_ID: 'test-wc-project-id' }, async () => {
      const loaded = await loadConfigFromFile({ command: 'build', mode: 'production' }, viteConfigPath)
      expect(loaded?.config.build?.sourcemap).toBe(false)
    })
  })

  it('enables build.sourcemap for non-production build modes', async () => {
    const loaded = await loadConfigFromFile({ command: 'build', mode: 'staging' }, viteConfigPath)
    expect(loaded?.config.build?.sourcemap).toBe(true)
  })

  it('rejects production build when VITE_DEV_MNEMONIC is set (GitLab #118)', async () => {
    const prev = process.env.VITE_DEV_MNEMONIC
    process.env.VITE_DEV_MNEMONIC = 'insecure-inline-for-test'
    try {
      await expect(loadConfigFromFile({ command: 'build', mode: 'production' }, viteConfigPath)).rejects.toThrow(
        /VITE_DEV_MNEMONIC must not be set/
      )
    } finally {
      if (prev === undefined) {
        delete process.env.VITE_DEV_MNEMONIC
      } else {
        process.env.VITE_DEV_MNEMONIC = prev
      }
    }
  })

  it('rejects staging build when VITE_DEV_MNEMONIC is set (GitLab #378)', async () => {
    const prevMnemonic = process.env.VITE_DEV_MNEMONIC
    const prevAllow = process.env.VITE_ALLOW_DEV_MNEMONIC
    process.env.VITE_DEV_MNEMONIC = 'insecure-inline-for-test'
    delete process.env.VITE_ALLOW_DEV_MNEMONIC
    try {
      await expect(loadConfigFromFile({ command: 'build', mode: 'staging' }, viteConfigPath)).rejects.toThrow(
        /VITE_DEV_MNEMONIC must not be set/
      )
    } finally {
      if (prevMnemonic === undefined) {
        delete process.env.VITE_DEV_MNEMONIC
      } else {
        process.env.VITE_DEV_MNEMONIC = prevMnemonic
      }
      if (prevAllow === undefined) {
        delete process.env.VITE_ALLOW_DEV_MNEMONIC
      } else {
        process.env.VITE_ALLOW_DEV_MNEMONIC = prevAllow
      }
    }
  })

  it('rejects production build when VITE_WC_PROJECT_ID is missing (GitLab #378)', async () => {
    const prev = process.env.VITE_WC_PROJECT_ID
    delete process.env.VITE_WC_PROJECT_ID
    try {
      await expect(loadConfigFromFile({ command: 'build', mode: 'production' }, viteConfigPath)).rejects.toThrow(
        /VITE_WC_PROJECT_ID is required/
      )
    } finally {
      if (prev === undefined) {
        delete process.env.VITE_WC_PROJECT_ID
      } else {
        process.env.VITE_WC_PROJECT_ID = prev
      }
    }
  })
})
