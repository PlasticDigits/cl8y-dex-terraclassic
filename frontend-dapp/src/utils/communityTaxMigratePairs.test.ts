import { describe, it, expect, vi, beforeEach } from 'vitest'
import {
  ALPHA_TOKEN_ADDRESS,
  ALPHA_TERRAPORT_LUNC_PAIR,
  ALPHA_TERRAPORT_USTC_PAIR,
  buildGovernanceTicket,
  knownTerraportRowsForToken,
  overlayKnownTerraportRows,
  pairContainsToken,
  registerCtaState,
  registerMigrateCl8yPair,
  tokenMatchesKnown,
  type Cl8yVenueRow,
  type OtherDexVenueRow,
} from './communityTaxMigratePairs'

const CL8Y_PAIR = 'terra1aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
const TERRAPORT_PAIR = ALPHA_TERRAPORT_LUNC_PAIR
const TOKEN = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const OPEN_LIKE = 'terra1qz56vxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxs74n3'

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/constants')>()
  return {
    ...actual,
    FACTORY_CONTRACT_ADDRESS: 'terra1factory',
    UST1_TOKEN_ADDRESS: '',
  }
})

vi.mock('@/services/indexer/client', () => ({
  getTokenPairs: vi.fn(),
}))

vi.mock('@/services/terraclassic/factory', () => ({
  getAllPairsPaginated: vi.fn(),
  getPair: vi.fn(),
  isCodeIdWhitelisted: vi.fn(),
}))

vi.mock('@/services/terraclassic/pair', () => ({
  getPool: vi.fn(),
}))

vi.mock('@/services/terraclassic/queries', () => ({
  getChainContractInfo: vi.fn(),
  queryContract: vi.fn(),
}))

vi.mock('@/services/terraclassic/assetCodeIdFreeze', () => ({
  probePairCodeIdFreeze: vi.fn(),
}))

vi.mock('@/services/terraclassic/communityTaxToken', () => ({
  queryCommunityTaxIsExempt: vi.fn(),
  registerListedPair: vi.fn(),
}))

vi.mock('@/utils/communityTaxRegisterPair', () => ({
  verifyFactoryListedPair: vi.fn(),
}))

function cl8y(over: Partial<Cl8yVenueRow> = {}): Cl8yVenueRow {
  return {
    venue: 'cl8y',
    pair: CL8Y_PAIR,
    symbols: ['GEM', 'LUNC'],
    otherAssetLabel: 'LUNC',
    otherAssetListed: true,
    frozen: false,
    registered: false,
    factoryVerified: true,
    ...over,
  }
}

const readyCtx = {
  postAdopt: true,
  isManager: true,
  taxPinMatches: true,
  adminIsCmm: true,
}

describe('communityTaxMigratePairs (#634)', () => {
  it('ALPHA always overlays both known Terraport rows', () => {
    const rows = knownTerraportRowsForToken(ALPHA_TOKEN_ADDRESS)
    expect(rows).toHaveLength(2)
    expect(rows.map((r) => r.pair)).toEqual([ALPHA_TERRAPORT_LUNC_PAIR, ALPHA_TERRAPORT_USTC_PAIR])
    const merged = overlayKnownTerraportRows(ALPHA_TOKEN_ADDRESS, [])
    expect(merged.every((r) => r.venue === 'other_dex')).toBe(true)
    expect(merged.every((r) => r.source === 'static')).toBe(true)
  })

  it('Open prefix/suffix matches the static Open/LUNC row', () => {
    expect(tokenMatchesKnown(OPEN_LIKE, knownTerraportRowsForToken(OPEN_LIKE)[0])).toBe(true)
    const merged = overlayKnownTerraportRows(OPEN_LIKE, [])
    expect(merged).toHaveLength(1)
    expect(merged[0].symbols).toEqual(['Open', 'LUNC'])
    expect(merged[0].venue).toBe('other_dex')
  })

  it('honest unknown token gets no static Terraport rows', () => {
    expect(knownTerraportRowsForToken(TOKEN)).toEqual([])
    expect(overlayKnownTerraportRows(TOKEN, [])).toEqual([])
  })

  it('overlay does not duplicate a factory-discovered ALPHA pair', () => {
    const discovered: OtherDexVenueRow[] = [
      {
        venue: 'other_dex',
        pair: ALPHA_TERRAPORT_LUNC_PAIR,
        symbols: ['ALPHA', 'LUNC'],
        source: 'terraport_factory',
      },
    ]
    const merged = overlayKnownTerraportRows(ALPHA_TOKEN_ADDRESS, discovered)
    expect(merged.filter((r) => r.pair === ALPHA_TERRAPORT_LUNC_PAIR)).toHaveLength(1)
    expect(merged.some((r) => r.pair === ALPHA_TERRAPORT_USTC_PAIR)).toBe(true)
  })

  it('pairContainsToken is CW20-only and case-insensitive', () => {
    expect(
      pairContainsToken(
        [{ token: { contract_addr: TOKEN.toUpperCase() } }, { native_token: { denom: 'uluna' } }],
        TOKEN
      )
    ).toBe(true)
    expect(
      pairContainsToken([{ token: { contract_addr: CL8Y_PAIR } }, { native_token: { denom: 'uluna' } }], TOKEN)
    ).toBe(false)
  })

  it('register CTA is hidden until post-adopt + manager + tax pin + CMM', () => {
    expect(registerCtaState(cl8y(), { ...readyCtx, postAdopt: false })).toBe('hidden')
    expect(registerCtaState(cl8y(), { ...readyCtx, isManager: false })).toBe('hidden')
    expect(registerCtaState(cl8y(), { ...readyCtx, taxPinMatches: false })).toBe('hidden')
    expect(registerCtaState(cl8y(), { ...readyCtx, adminIsCmm: false })).toBe('hidden')
    expect(registerCtaState(cl8y(), readyCtx)).toBe('ready')
  })

  it('register waits while frozen or unknown F6; skips unlisted other; hides when already', () => {
    expect(registerCtaState(cl8y({ frozen: true }), readyCtx)).toBe('wait_refresh')
    expect(registerCtaState(cl8y({ frozen: 'unknown' }), readyCtx)).toBe('wait_refresh')
    expect(registerCtaState(cl8y({ otherAssetListed: false }), readyCtx)).toBe('skip_unlisted')
    expect(registerCtaState(cl8y({ registered: true }), readyCtx)).toBe('already')
  })

  it('governance ticket names both CL8Y pairs and keeps Refresh off the retail card path', () => {
    const text = buildGovernanceTicket(TOKEN, [
      cl8y({ pair: CL8Y_PAIR, symbols: ['GEM', 'LUNC'] }),
      cl8y({ pair: 'terra1bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb', symbols: ['GEM', 'UST1'] }),
    ])
    expect(text).toContain(TOKEN)
    expect(text).toContain(CL8Y_PAIR)
    expect(text).toMatch(/RefreshPair after 11619 adopt/)
    expect(text).toContain('terra1bbbb')
  })

  it('governance ticket tells ops to skip Refresh when the other asset is unlisted', () => {
    const text = buildGovernanceTicket(TOKEN, [cl8y({ otherAssetListed: false, otherAssetLabel: 'GEM2' })])
    expect(text).toMatch(/Skip Refresh/)
    expect(text).not.toMatch(/RefreshPair after 11619 adopt/)
  })
})

describe('loadMigratePairInventory (#634)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('indexer miss still finds a factory-verified CL8Y pair', async () => {
    const { getTokenPairs } = await import('@/services/indexer/client')
    const { getAllPairsPaginated, getPair, isCodeIdWhitelisted } = await import('@/services/terraclassic/factory')
    const { getPool } = await import('@/services/terraclassic/pair')
    const { queryContract } = await import('@/services/terraclassic/queries')
    const { verifyFactoryListedPair } = await import('@/utils/communityTaxRegisterPair')
    vi.mocked(getTokenPairs).mockRejectedValue(new Error('indexer down'))
    vi.mocked(getAllPairsPaginated).mockResolvedValue({
      pairs: [
        {
          contract_addr: CL8Y_PAIR,
          asset_infos: [{ token: { contract_addr: TOKEN } }, { native_token: { denom: 'uluna' } }],
          liquidity_token: 'lp',
        },
      ],
    })
    vi.mocked(getPair).mockRejectedValue(new Error('no pair'))
    vi.mocked(verifyFactoryListedPair).mockResolvedValue({
      contract_addr: CL8Y_PAIR,
      asset_infos: [{ token: { contract_addr: TOKEN } }, { native_token: { denom: 'uluna' } }],
      liquidity_token: 'lp',
    })
    vi.mocked(getPool).mockResolvedValue({
      assets: [
        { info: { token: { contract_addr: TOKEN } }, amount: '1' },
        { info: { native_token: { denom: 'uluna' } }, amount: '2' },
      ],
      total_share: '1',
    })
    vi.mocked(isCodeIdWhitelisted).mockResolvedValue({ code_id: 1, whitelisted: true })
    vi.mocked(queryContract).mockRejectedValue(new Error('terraport down'))

    const { loadMigratePairInventory } = await import('./communityTaxMigratePairs')
    const inv = await loadMigratePairInventory(TOKEN)
    expect(inv.cl8y.map((r) => r.pair)).toEqual([CL8Y_PAIR])
    expect(inv.cl8y[0].factoryVerified).toBe(true)
    expect(inv.terraportIncomplete).toBe(true)
  })

  it('failed Terraport query keeps static ALPHA rows and stays submitable', async () => {
    const { getTokenPairs } = await import('@/services/indexer/client')
    const { getAllPairsPaginated, getPair } = await import('@/services/terraclassic/factory')
    const { queryContract } = await import('@/services/terraclassic/queries')
    vi.mocked(getTokenPairs).mockResolvedValue([])
    vi.mocked(getAllPairsPaginated).mockResolvedValue({ pairs: [] })
    vi.mocked(getPair).mockRejectedValue(new Error('no pair'))
    vi.mocked(queryContract).mockRejectedValue(new Error('lcd timeout'))

    const { loadMigratePairInventory } = await import('./communityTaxMigratePairs')
    const inv = await loadMigratePairInventory(ALPHA_TOKEN_ADDRESS)
    expect(inv.cl8y).toEqual([])
    expect(inv.otherDex).toHaveLength(2)
    expect(inv.terraportIncomplete).toBe(true)
  })
})

describe('registerMigrateCl8yPair (#634)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('refuses a non-factory pair and never calls registerListedPair', async () => {
    const { verifyFactoryListedPair } = await import('@/utils/communityTaxRegisterPair')
    const { registerListedPair } = await import('@/services/terraclassic/communityTaxToken')
    vi.mocked(verifyFactoryListedPair).mockResolvedValue(null)
    await expect(
      registerMigrateCl8yPair({ wallet: 'terra1wallet', token: TOKEN, pair: TERRAPORT_PAIR })
    ).rejects.toThrow(/not a CL8Y market/)
    expect(registerListedPair).not.toHaveBeenCalled()
  })

  it('registers only the factory-verified contract addr', async () => {
    const { verifyFactoryListedPair } = await import('@/utils/communityTaxRegisterPair')
    const { registerListedPair } = await import('@/services/terraclassic/communityTaxToken')
    vi.mocked(verifyFactoryListedPair).mockResolvedValue({
      contract_addr: CL8Y_PAIR,
      asset_infos: [{ token: { contract_addr: TOKEN } }, { native_token: { denom: 'uluna' } }],
      liquidity_token: 'lp',
    })
    vi.mocked(registerListedPair).mockResolvedValue('reg-tx')
    await expect(registerMigrateCl8yPair({ wallet: 'terra1wallet', token: TOKEN, pair: CL8Y_PAIR })).resolves.toBe(
      'reg-tx'
    )
    expect(registerListedPair).toHaveBeenCalledWith('terra1wallet', TOKEN, CL8Y_PAIR)
  })
})
