import { describe, expect, it } from 'vitest'
import { buildProductionConnectSrc } from '../cspConnectSrc'

describe('buildProductionConnectSrc', () => {
  it('narrows connect-src to env-specific hosts without broad https:', () => {
    const connectSrc = buildProductionConnectSrc({
      VITE_TERRA_LCD_URL: 'https://lcd.example.com',
      VITE_TERRA_RPC_URL: 'https://rpc.example.com:443',
      VITE_INDEXER_URL: 'https://indexer.example.com',
    })
    expect(connectSrc).toContain("'self'")
    expect(connectSrc).toContain('https://lcd.example.com')
    expect(connectSrc).toContain('https://indexer.example.com')
    expect(connectSrc).toContain('https://rpc.example.com')
    expect(connectSrc).toContain('wss://rpc.example.com')
    expect(connectSrc).not.toMatch(/\bhttps:\s/)
    expect(connectSrc).not.toMatch(/\bwss:\s/)
  })

  it('includes WalletConnect relay hosts', () => {
    const connectSrc = buildProductionConnectSrc({})
    expect(connectSrc).toContain('https://relay.walletconnect.com')
    expect(connectSrc).toContain('wss://relay.walletconnect.com')
  })
})
