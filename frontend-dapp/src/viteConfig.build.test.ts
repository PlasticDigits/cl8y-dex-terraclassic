/** @vitest-environment node */
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { loadConfigFromFile } from 'vite'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

const viteConfigPath = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', 'vite.config.ts')

describe('vite.config production source maps', () => {
  let prevWc: string | undefined
  let prevDevMode: string | undefined

  beforeEach(() => {
    prevWc = process.env.VITE_WC_PROJECT_ID
    prevDevMode = process.env.VITE_DEV_MODE
    process.env.VITE_WC_PROJECT_ID = process.env.VITE_WC_PROJECT_ID || 'vitest-wc-project-id'
    // Override vitest.config test.env + any copied .env so production loadConfigFromFile stays hermetic (GitLab #695).
    process.env.VITE_DEV_MODE = 'false'
  })

  afterEach(() => {
    if (prevWc === undefined) {
      delete process.env.VITE_WC_PROJECT_ID
    } else {
      process.env.VITE_WC_PROJECT_ID = prevWc
    }
    if (prevDevMode === undefined) {
      delete process.env.VITE_DEV_MODE
    } else {
      process.env.VITE_DEV_MODE = prevDevMode
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

  it('rejects staging build when VITE_DEV_MNEMONIC is set without local-only escape (GitLab #378)', async () => {
    const prevMnemonic = process.env.VITE_DEV_MNEMONIC
    const prevAllow = process.env.VITE_ALLOW_DEV_MNEMONIC
    process.env.VITE_DEV_MNEMONIC = 'insecure-inline-for-test'
    delete process.env.VITE_ALLOW_DEV_MNEMONIC
    try {
      await expect(loadConfigFromFile({ command: 'build', mode: 'staging' }, viteConfigPath)).rejects.toThrow(
        /VITE_DEV_MNEMONIC must not be set/
      )
    } finally {
      if (prevMnemonic === undefined) delete process.env.VITE_DEV_MNEMONIC
      else process.env.VITE_DEV_MNEMONIC = prevMnemonic
      if (prevAllow === undefined) delete process.env.VITE_ALLOW_DEV_MNEMONIC
      else process.env.VITE_ALLOW_DEV_MNEMONIC = prevAllow
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

  it('rejects production build when VITE_DEV_MODE is true (GitLab #695)', async () => {
    process.env.VITE_DEV_MODE = 'true'
    await expect(loadConfigFromFile({ command: 'build', mode: 'production' }, viteConfigPath)).rejects.toThrow(
      /VITE_DEV_MODE/
    )
  })

  it('loads production config when VITE_DEV_MODE is unset or false (GitLab #695)', async () => {
    // Vite loadEnv fills VITE_* from .env.local (LocalTerra / leftover worktree copy often
    // has VITE_DEV_MODE=true). process.env wins only when the key stays set — deleting it
    // lets dotenv re-inject `true` and the production reject fires. Empty string is not
    // === 'true' (GitLab #698 leftover / D695 hermetic loadConfigFromFile).
    process.env.VITE_DEV_MODE = ''
    const unset = await loadConfigFromFile({ command: 'build', mode: 'production' }, viteConfigPath)
    expect(unset?.config.build?.sourcemap).toBe(false)

    process.env.VITE_DEV_MODE = 'false'
    const flaggedFalse = await loadConfigFromFile({ command: 'build', mode: 'production' }, viteConfigPath)
    expect(flaggedFalse?.config.build?.sourcemap).toBe(false)
  })

  it('loads development build when VITE_DEV_MODE is true (GitLab #695)', async () => {
    process.env.VITE_DEV_MODE = 'true'
    const loaded = await loadConfigFromFile({ command: 'build', mode: 'development' }, viteConfigPath)
    expect(loaded?.config).toBeDefined()
  })

  it('loads staging build when VITE_DEV_MODE is true (GitLab #695)', async () => {
    process.env.VITE_DEV_MODE = 'true'
    const loaded = await loadConfigFromFile({ command: 'build', mode: 'staging' }, viteConfigPath)
    expect(loaded?.config.build?.sourcemap).toBe(true)
  })

  it('does not run build guards for serve even in production mode (GitLab #695)', async () => {
    process.env.VITE_DEV_MODE = 'true'
    const loaded = await loadConfigFromFile({ command: 'serve', mode: 'production' }, viteConfigPath)
    expect(loaded?.config).toBeDefined()
  })

  it('still rejects production VITE_DEV_MODE when local-only mnemonic escape is set (GitLab #695)', async () => {
    process.env.VITE_DEV_MODE = 'true'
    process.env.VITE_ALLOW_DEV_MNEMONIC = 'local-only'
    try {
      await expect(loadConfigFromFile({ command: 'build', mode: 'production' }, viteConfigPath)).rejects.toThrow(
        /VITE_DEV_MODE/
      )
    } finally {
      delete process.env.VITE_ALLOW_DEV_MNEMONIC
    }
  })

  it('rejects production mnemonic before or instead of VITE_DEV_MODE when both are set (GitLab #695)', async () => {
    process.env.VITE_DEV_MODE = 'true'
    process.env.VITE_DEV_MNEMONIC = 'insecure-inline-for-test'
    try {
      await expect(loadConfigFromFile({ command: 'build', mode: 'production' }, viteConfigPath)).rejects.toThrow(
        /VITE_DEV_MNEMONIC must not be set/
      )
    } finally {
      delete process.env.VITE_DEV_MNEMONIC
    }
  })
})
