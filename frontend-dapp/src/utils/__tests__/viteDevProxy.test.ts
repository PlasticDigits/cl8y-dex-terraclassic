import { describe, expect, it } from 'vitest'
import { DEV_PROXY_INDEXER_PREFIX, DEV_PROXY_LCD_PREFIX, isRemoteHttpUrl, planDevRemoteProxy } from '@/dev/viteDevProxy'

describe('viteDevProxy', () => {
  it('detects remote vs loopback URLs', () => {
    expect(isRemoteHttpUrl('https://indexer.dex.cl8y.com')).toBe(true)
    expect(isRemoteHttpUrl('https://terra-classic-lcd.publicnode.com')).toBe(true)
    expect(isRemoteHttpUrl('http://127.0.0.1:3001')).toBe(false)
    expect(isRemoteHttpUrl('http://localhost:1317')).toBe(false)
    expect(isRemoteHttpUrl('/__dev/indexer')).toBe(false)
  })

  it('auto-proxies soft-launch remote indexer + LCD', () => {
    const plan = planDevRemoteProxy({
      VITE_INDEXER_URL: 'https://indexer.dex.cl8y.com',
      VITE_TERRA_LCD_URL: 'https://terra-classic-lcd.publicnode.com',
    })
    expect(plan.indexerTarget).toBe('https://indexer.dex.cl8y.com')
    expect(plan.lcdTarget).toBe('https://terra-classic-lcd.publicnode.com')
    expect(plan.indexerBrowserUrl).toBe(DEV_PROXY_INDEXER_PREFIX)
    expect(plan.lcdBrowserUrl).toBe(DEV_PROXY_LCD_PREFIX)
  })

  it('leaves LocalTerra loopback URLs alone', () => {
    const plan = planDevRemoteProxy({
      VITE_INDEXER_URL: 'http://127.0.0.1:3001',
      VITE_TERRA_LCD_URL: 'http://127.0.0.1:1317',
    })
    expect(plan.indexerTarget).toBeNull()
    expect(plan.lcdTarget).toBeNull()
  })

  it('respects VITE_DEV_PROXY=0 even for remote URLs', () => {
    const plan = planDevRemoteProxy({
      VITE_DEV_PROXY: '0',
      VITE_INDEXER_URL: 'https://indexer.dex.cl8y.com',
      VITE_TERRA_LCD_URL: 'https://terra-classic-lcd.publicnode.com',
    })
    expect(plan.indexerTarget).toBeNull()
    expect(plan.lcdTarget).toBeNull()
  })
})
