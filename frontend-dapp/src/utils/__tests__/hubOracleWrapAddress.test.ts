import { afterEach, describe, expect, it, vi } from 'vitest'

const CUSTC = 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch'
const CLUNC = 'terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg'

describe('resolveHubOracleWrapAddress (GitLab #570)', () => {
  afterEach(() => {
    vi.unstubAllEnvs()
    vi.resetModules()
  })

  it('prefers valid env overlay over spoofed API asset_address', async () => {
    vi.stubEnv('VITE_NETWORK', 'mainnet')
    vi.stubEnv('VITE_USTC_C_TOKEN_ADDRESS', CUSTC)
    vi.stubEnv('VITE_LUNC_C_TOKEN_ADDRESS', CLUNC)
    vi.resetModules()
    const { resolveHubOracleWrapAddress } = await import('../hubOracleWrapAddress')
    expect(resolveHubOracleWrapAddress('custc', 'javascript:alert(1)')).toBe(CUSTC)
    expect(resolveHubOracleWrapAddress('lunc', 'https://evil.example/')).toBe(CLUNC)
    expect(resolveHubOracleWrapAddress('ust1', CUSTC)).toBeNull()
    expect(resolveHubOracleWrapAddress('ustr', CLUNC)).toBeNull()
  })

  it('omits the row when configured env is not allowlisted bech32', async () => {
    vi.stubEnv('VITE_NETWORK', 'mainnet')
    vi.stubEnv('VITE_USTC_C_TOKEN_ADDRESS', ' javascript:alert(1)')
    vi.stubEnv('VITE_LUNC_C_TOKEN_ADDRESS', 'uluna')
    vi.resetModules()
    const { resolveHubOracleWrapAddress } = await import('../hubOracleWrapAddress')
    expect(resolveHubOracleWrapAddress('custc', CUSTC)).toBeNull()
    expect(resolveHubOracleWrapAddress('lunc', CLUNC)).toBeNull()
  })

  it('falls back to API asset_address only when env is empty and URL-safe', async () => {
    vi.stubEnv('VITE_NETWORK', 'mainnet')
    vi.stubEnv('VITE_USTC_C_TOKEN_ADDRESS', '')
    vi.stubEnv('VITE_LUNC_C_TOKEN_ADDRESS', '')
    vi.resetModules()
    const { resolveHubOracleWrapAddress } = await import('../hubOracleWrapAddress')
    expect(resolveHubOracleWrapAddress('custc', CUSTC)).toBe(CUSTC)
    expect(resolveHubOracleWrapAddress('lunc', CLUNC)).toBe(CLUNC)
    expect(resolveHubOracleWrapAddress('custc', 'javascript:alert(1)')).toBeNull()
    expect(resolveHubOracleWrapAddress('lunc', 'data:text/html,x')).toBeNull()
    expect(resolveHubOracleWrapAddress('custc', '../terra1')).toBeNull()
    expect(resolveHubOracleWrapAddress('lunc', '')).toBeNull()
  })
})
