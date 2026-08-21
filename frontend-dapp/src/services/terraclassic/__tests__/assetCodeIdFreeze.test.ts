import { describe, expect, it, vi, beforeEach } from 'vitest'
import { probePairCodeIdFreeze } from '../assetCodeIdFreeze'
import { getAssetCodeIds, getPairInfo } from '../pair'
import { getChainContractInfo } from '../queries'
import { isCodeIdWhitelisted } from '../factory'

vi.mock('../pair', () => ({
  getAssetCodeIds: vi.fn(),
  getPairInfo: vi.fn(),
}))
vi.mock('../queries', () => ({
  getChainContractInfo: vi.fn(),
}))
vi.mock('../factory', () => ({
  isCodeIdWhitelisted: vi.fn(),
}))

const PAIR = 'terra1pair00000000000000000000000000000001'
const T0 = 'terra1token000000000000000000000000000001'
const T1 = 'terra1token000000000000000000000000000002'

describe('probePairCodeIdFreeze (GitLab #585)', () => {
  beforeEach(() => {
    vi.mocked(getPairInfo).mockResolvedValue({
      contract_addr: PAIR,
      liquidity_token: 'terra1lp',
      asset_infos: [{ token: { contract_addr: T0 } }, { token: { contract_addr: T1 } }],
    })
    vi.mocked(getChainContractInfo).mockImplementation(async (addr: string) => ({
      code_id: addr === T0 ? 10184 : 6036,
      creator: 'terra1x',
      admin: '',
      label: 't',
    }))
    vi.mocked(isCodeIdWhitelisted).mockResolvedValue({ code_id: 10184, whitelisted: true })
  })

  it('returns frozen when live code_id drifted from pin', async () => {
    vi.mocked(getAssetCodeIds).mockResolvedValue({ code_ids: [10184, 6036] })
    vi.mocked(getChainContractInfo).mockResolvedValueOnce({
      code_id: 9999,
      creator: '',
      admin: '',
      label: '',
    })
    vi.mocked(getChainContractInfo).mockResolvedValueOnce({
      code_id: 6036,
      creator: '',
      admin: '',
      label: '',
    })
    const out = await probePairCodeIdFreeze(PAIR)
    expect(out.frozen).toBe(true)
    expect(out.verdict).toBe('frozen')
  })

  it('returns tradable when pins match and whitelist holds', async () => {
    vi.mocked(getAssetCodeIds).mockResolvedValue({ code_ids: [10184, 6036] })
    const out = await probePairCodeIdFreeze(PAIR)
    expect(out.frozen).toBe(false)
    expect(out.verdict).toBe('tradable')
  })

  it('treats pre-1.15.0 unknown variant as tradable (not frozen)', async () => {
    vi.mocked(getAssetCodeIds).mockRejectedValue(new Error('unknown variant `get_asset_code_ids`'))
    const out = await probePairCodeIdFreeze(PAIR)
    expect(out.frozen).toBe(false)
    expect(out.verdict).toBe('tradable')
  })

  it('fail-opens on LCD timeout', async () => {
    vi.mocked(getAssetCodeIds).mockRejectedValue(new Error('LCD request timed out after 10000ms'))
    const out = await probePairCodeIdFreeze(PAIR)
    expect(out.frozen).toBe(false)
    expect(out.verdict).toBe('unknown')
  })
})
