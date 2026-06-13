/** @vitest-environment node */
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { loadConfigFromFile } from 'vite'
import { describe, expect, it } from 'vitest'
import { assertNonDevelopmentBuildMnemonic, assertProductionBuildEnv, buildProductionCsp } from '../vite.config'

const viteConfigPath = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', 'vite.config.ts')

describe('vite.config production source maps', () => {
  it('disables build.sourcemap for production mode (GitLab #117)', async () => {
    const prevWc = process.env.VITE_WC_PROJECT_ID
    process.env.VITE_WC_PROJECT_ID = 'test-wc-project-id'
    try {
      const loaded = await loadConfigFromFile({ command: 'build', mode: 'production' }, viteConfigPath)
      expect(loaded?.config.build?.sourcemap).toBe(false)
    } finally {
      if (prevWc === undefined) delete process.env.VITE_WC_PROJECT_ID
      else process.env.VITE_WC_PROJECT_ID = prevWc
    }
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
    const prev = process.env.VITE_WC_PROJECT_ID
    delete process.env.VITE_WC_PROJECT_ID
    try {
      await expect(loadConfigFromFile({ command: 'build', mode: 'production' }, viteConfigPath)).rejects.toThrow(
        /VITE_WC_PROJECT_ID must be set/
      )
    } finally {
      if (prev === undefined) delete process.env.VITE_WC_PROJECT_ID
      else process.env.VITE_WC_PROJECT_ID = prev
    }
  })
})

describe('vite.config production CSP (GitLab #378)', () => {
  it('narrows connect-src to env hosts and WalletConnect (no broad https:)', () => {
    const csp = buildProductionCsp({
      VITE_TERRA_LCD_URL: 'https://terra-classic-lcd.publicnode.com',
      VITE_TERRA_RPC_URL: 'https://terra-classic-rpc.publicnode.com:443',
      VITE_INDEXER_URL: 'https://indexer.example.com',
    })
    expect(csp).toContain('https://indexer.example.com')
    expect(csp).toContain('wss://relay.walletconnect.com')
    const connect = csp.match(/connect-src ([^;]+)/)?.[1] ?? ''
    expect(connect.split(/\s+/)).not.toContain('https:')
    expect(connect.split(/\s+/)).not.toContain('wss:')
  })

  it('uses script-src self without unsafe-inline in production', () => {
    const csp = buildProductionCsp({})
    const script = csp.match(/script-src ([^;]+)/)?.[1] ?? ''
    expect(script).toBe("'self'")
    expect(script).not.toContain('unsafe-inline')
  })
})

describe('vite.config build env guards (GitLab #378)', () => {
  it('assertNonDevelopmentBuildMnemonic allows development mode', () => {
    expect(() =>
      assertNonDevelopmentBuildMnemonic('development', { VITE_DEV_MNEMONIC: 'seed words here' })
    ).not.toThrow()
  })

  it('assertNonDevelopmentBuildMnemonic allows local-only escape', () => {
    expect(() =>
      assertNonDevelopmentBuildMnemonic('staging', {
        VITE_DEV_MNEMONIC: 'seed words here',
        VITE_ALLOW_DEV_MNEMONIC: 'local-only',
      })
    ).not.toThrow()
  })

  it('assertProductionBuildEnv requires WC project id', () => {
    expect(() => assertProductionBuildEnv('production', {})).toThrow(/VITE_WC_PROJECT_ID/)
  })
})
