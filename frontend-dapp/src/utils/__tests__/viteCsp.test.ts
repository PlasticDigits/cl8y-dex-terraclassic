/** @vitest-environment node */
import { readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import {
  buildProductionCspMetaContent,
  DEV_CSP_META_CONTENT,
  PRODUCTION_TERRA_LCD_FALLBACK,
  PRODUCTION_TERRA_RPC_FALLBACK,
} from '../../../viteCsp'

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../../..')

describe('viteCsp production policy', () => {
  it('includes constants.ts LCD/RPC fallbacks when env URLs are unset', () => {
    const csp = buildProductionCspMetaContent({})
    expect(csp).toContain(PRODUCTION_TERRA_LCD_FALLBACK)
    expect(csp).toContain(PRODUCTION_TERRA_RPC_FALLBACK)
  })

  it('narrows connect-src to env hosts (no blanket https:)', () => {
    const csp = buildProductionCspMetaContent({
      VITE_TERRA_LCD_URL: 'https://terra-classic-lcd.example.com',
      VITE_TERRA_RPC_URL: 'https://terra-classic-rpc.example.com:443',
      VITE_INDEXER_URL: 'https://indexer.example.com',
    })
    expect(csp).toContain('connect-src')
    expect(csp).toContain('https://terra-classic-lcd.example.com')
    expect(csp).toContain('https://indexer.example.com')
    expect(csp).not.toMatch(/connect-src[^;]*\shttps:\s/)
  })

  it('documents dev exception separately', () => {
    expect(DEV_CSP_META_CONTENT).toContain('https:')
    expect(DEV_CSP_META_CONTENT).toContain('http://127.0.0.1:*')
  })

  it('render.yaml omits static CSP header (env-aware meta policy only)', () => {
    const renderYaml = readFileSync(path.join(repoRoot, 'render.yaml'), 'utf8')
    expect(renderYaml).not.toMatch(/^\s+name:\s+Content-Security-Policy/m)
  })
})
