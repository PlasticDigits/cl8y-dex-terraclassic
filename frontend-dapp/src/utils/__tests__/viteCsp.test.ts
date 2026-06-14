/** @vitest-environment node */
import { describe, expect, it } from 'vitest'
import { buildProductionCspMetaContent, DEV_CSP_META_CONTENT } from '../../../viteCsp'

describe('viteCsp production policy', () => {
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
})
