/** @vitest-environment node */
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { loadConfigFromFile } from 'vite'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

const viteConfigPath = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', 'vite.config.ts')

describe('vite.config production source maps', () => {
  let prevWc: string | undefined

  beforeEach(() => {
    prevWc = process.env.VITE_WC_PROJECT_ID
    process.env.VITE_WC_PROJECT_ID = process.env.VITE_WC_PROJECT_ID || 'vitest-wc-project-id'
  })

  afterEach(() => {
    if (prevWc === undefined) {
      delete process.env.VITE_WC_PROJECT_ID
    } else {
      process.env.VITE_WC_PROJECT_ID = prevWc
    }
  })

  it('disables build.sourcemap for production mode (GitLab #117)', async () => {
    const loaded = await loadConfigFromFile({ command: 'build', mode: 'production' }, viteConfigPath)
    expect(loaded?.config.build?.sourcemap).toBe(false)
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
    const prev = process.env.VITE_DEV_MNEMONIC
    process.env.VITE_DEV_MNEMONIC = 'insecure-inline-for-test'
    try {
      await expect(loadConfigFromFile({ command: 'build', mode: 'staging' }, viteConfigPath)).rejects.toThrow(
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

  it('rejects production build when VITE_WC_PROJECT_ID is missing (GitLab #378)', async () => {
    const prevMnemonic = process.env.VITE_DEV_MNEMONIC
    const prevWcLocal = process.env.VITE_WC_PROJECT_ID
    delete process.env.VITE_DEV_MNEMONIC
    delete process.env.VITE_WC_PROJECT_ID
    try {
      await expect(loadConfigFromFile({ command: 'build', mode: 'production' }, viteConfigPath)).rejects.toThrow(
        /VITE_WC_PROJECT_ID is required/
      )
    } finally {
      if (prevMnemonic === undefined) {
        delete process.env.VITE_DEV_MNEMONIC
      } else {
        process.env.VITE_DEV_MNEMONIC = prevMnemonic
      }
      if (prevWcLocal === undefined) {
        delete process.env.VITE_WC_PROJECT_ID
      } else {
        process.env.VITE_WC_PROJECT_ID = prevWcLocal
      }
    }
  })
})
