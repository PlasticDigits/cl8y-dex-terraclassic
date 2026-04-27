import { mkdirSync, writeFileSync, rmSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { describe, it, expect, afterEach } from 'vitest'
import { loadViteEnv } from './env.js'

describe('loadViteEnv', () => {
  let dir = ''
  afterEach(() => {
    if (dir) rmSync(dir, { recursive: true, force: true })
  })

  it('reads VITE_* from .env.local and allows process.env override', () => {
    dir = join(tmpdir(), `swarm-env-${Date.now()}`)
    mkdirSync(join(dir, 'frontend-dapp'), { recursive: true })
    writeFileSync(
      join(dir, 'frontend-dapp', '.env.local'),
      `
# comment
VITE_FACTORY_ADDRESS=terra111
VITE_ROUTER_ADDRESS=terra222
VITE_TERRA_LCD_URL=http://localhost:1317
VITE_TERRA_RPC_URL=http://localhost:26657
VITE_NETWORK=local
`
    )
    process.env.VITE_FACTORY_ADDRESS = 'terra999override'
    const e = loadViteEnv(dir)
    expect(e.VITE_FACTORY_ADDRESS).toBe('terra999override')
    expect(e.VITE_ROUTER_ADDRESS).toBe('terra222')
    delete process.env.VITE_FACTORY_ADDRESS
  })
})
