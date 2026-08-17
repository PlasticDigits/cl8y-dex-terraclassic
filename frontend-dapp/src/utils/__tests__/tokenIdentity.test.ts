import { afterEach, describe, expect, it, vi } from 'vitest'
import type { AssetInfo, IndexerAssetBrief } from '@/types'
import { MAINNET_CUSTC_TOKEN_ADDRESS, MAINNET_UST1_TOKEN_ADDRESS } from '@/utils/ust1SecondaryMarket'

const CW20_A = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const CW20_B = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
const BAD_CHECKSUM = 'terra1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx'

async function loadIdentity() {
  return import('../tokenIdentity')
}

describe('tokenIdentityTarget (GitLab #541)', () => {
  afterEach(() => {
    vi.unstubAllEnvs()
    vi.resetModules()
  })

  it('T1: valid CW20 returns address + explorer URL per VITE_NETWORK', async () => {
    vi.stubEnv('VITE_NETWORK', 'mainnet')
    vi.resetModules()
    const { tokenIdentityTarget } = await loadIdentity()
    const { getExplorerAddressUrl } = await import('@/utils/terraExplorer')
    const info: AssetInfo = { token: { contract_addr: CW20_A } }
    const target = tokenIdentityTarget(info)
    expect(target).toEqual({
      kind: 'cw20',
      address: CW20_A,
      explorerUrl: `https://finder.terraclassic.community/columbus-5/address/${CW20_A}`,
    })
    expect(target && target.kind === 'cw20' ? target.explorerUrl : null).toBe(getExplorerAddressUrl(CW20_A))
  })

  it('T1: testnet Hexxagon + local LCD account paths', async () => {
    vi.stubEnv('VITE_NETWORK', 'testnet')
    vi.resetModules()
    let { tokenIdentityTarget } = await loadIdentity()
    expect(tokenIdentityTarget({ token: { contract_addr: CW20_A } })).toMatchObject({
      kind: 'cw20',
      address: CW20_A,
      explorerUrl: `https://finder.terra-classic.hexxagon.io/testnet/address/${CW20_A}`,
    })

    vi.stubEnv('VITE_NETWORK', 'local')
    vi.resetModules()
    ;({ tokenIdentityTarget } = await loadIdentity())
    expect(tokenIdentityTarget({ token: { contract_addr: CW20_A } })).toMatchObject({
      kind: 'cw20',
      address: CW20_A,
      explorerUrl: `http://localhost:1317/cosmos/auth/v1beta1/accounts/${CW20_A}`,
    })
  })

  it('T2: native uluna / uusd are copy-only (no URL)', async () => {
    const { tokenIdentityTarget } = await loadIdentity()
    expect(tokenIdentityTarget({ native_token: { denom: 'uluna' } })).toEqual({ kind: 'native', denom: 'uluna' })
    expect(tokenIdentityTarget({ native_token: { denom: 'uusd' } })).toEqual({ kind: 'native', denom: 'uusd' })
  })

  it('T3 / A1: invalid, empty, javascript, HTML, bad checksum → null', async () => {
    const { tokenIdentityTarget } = await loadIdentity()
    expect(tokenIdentityTarget(null)).toBeNull()
    expect(tokenIdentityTarget({ token: { contract_addr: '' } })).toBeNull()
    expect(tokenIdentityTarget({ token: { contract_addr: 'javascript:alert(1)' } })).toBeNull()
    expect(tokenIdentityTarget({ token: { contract_addr: '<script>x</script>' } })).toBeNull()
    expect(tokenIdentityTarget({ token: { contract_addr: BAD_CHECKSUM } })).toBeNull()
    expect(tokenIdentityTarget({ native_token: { denom: 'javascript:alert(1)' } })).toBeNull()
    expect(tokenIdentityTarget({ native_token: { denom: 'uluna"><img' } })).toBeNull()
    expect(tokenIdentityTarget({ native_token: { denom: '' } })).toBeNull()
  })

  it('T4 / A4: spoofed UST1 symbol still copies the attacker contract', async () => {
    const { tokenIdentityTargetFromIndexerBrief, copyPayload } = await loadIdentity()
    const brief: IndexerAssetBrief = {
      symbol: 'UST1',
      contract_addr: CW20_B,
      denom: null,
      decimals: 6,
    }
    const target = tokenIdentityTargetFromIndexerBrief(brief)
    expect(target?.kind).toBe('cw20')
    expect(copyPayload(target!)).toBe(CW20_B)
    expect(copyPayload(target!)).not.toBe('UST1')
    if (target?.kind === 'cw20') {
      expect(target.explorerUrl).toContain(CW20_B)
      expect(target.explorerUrl).not.toContain('UST1')
    }
  })

  it('T5 / A7: invert flips display order, not factory targets', async () => {
    const { pairIdentityTargets, pairIdentityLegOrder } = await loadIdentity()
    const asset0: AssetInfo = { token: { contract_addr: MAINNET_UST1_TOKEN_ADDRESS } }
    const asset1: AssetInfo = { token: { contract_addr: MAINNET_CUSTC_TOKEN_ADDRESS } }
    const targets = pairIdentityTargets({ asset0, asset1 })
    expect(targets.base && targets.base.kind === 'cw20' ? targets.base.address : null).toBe(MAINNET_UST1_TOKEN_ADDRESS)
    expect(targets.quote && targets.quote.kind === 'cw20' ? targets.quote.address : null).toBe(
      MAINNET_CUSTC_TOKEN_ADDRESS
    )
    expect(pairIdentityLegOrder(false)).toEqual(['base', 'quote'])
    expect(pairIdentityLegOrder(true)).toEqual(['quote', 'base'])
    const inverted = pairIdentityTargets({ asset0, asset1 })
    expect(inverted).toEqual(targets)
  })

  it('A6: native target has no explorer URL field', async () => {
    const { tokenIdentityTarget } = await loadIdentity()
    const target = tokenIdentityTarget({ native_token: { denom: 'uluna' } })
    expect(target).toEqual({ kind: 'native', denom: 'uluna' })
    expect(target && 'explorerUrl' in target).toBe(false)
  })
})
