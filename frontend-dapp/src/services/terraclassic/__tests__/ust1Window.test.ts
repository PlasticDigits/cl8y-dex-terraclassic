import { beforeEach, describe, expect, it, vi } from 'vitest'

const { WINDOW, UST1, VFDUSD, WALLET } = vi.hoisted(() => ({
  WINDOW: 'terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2',
  UST1: 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72',
  VFDUSD: 'terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3',
  WALLET: 'terra1wallet00000000000000000000000000001',
}))

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/constants')>()
  return {
    ...actual,
    UST1_WINDOW_CONTRACT_ADDRESS: WINDOW,
    UST1_TOKEN_ADDRESS: UST1,
    VFDUSD_TOKEN_ADDRESS: VFDUSD,
  }
})

vi.mock('../queries', () => ({
  queryContract: vi.fn(),
}))

vi.mock('../transactions', () => ({
  executeTerraContract: vi.fn(),
}))

import { queryContract } from '../queries'
import { executeTerraContract } from '../transactions'
import {
  depositVfdusdForUst1,
  executeUst1Window,
  getUst1EffectiveSwap,
  payTokenForDirection,
  withdrawUst1ForVfdusd,
} from '../ust1Window'
import { UST1_RATE_SCALE } from '@/utils/ust1WindowMath'

describe('ust1Window client (#506)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('queries effective_swap on the window', async () => {
    vi.mocked(queryContract).mockResolvedValue({
      fee_bps: 100,
      per_tx_ust1_limit: '1000000000',
      rolling_24h_ust1_limit: '10000000000',
      paused: false,
      rolling_window_start_sec: 0,
      rolling_volume_ust1: '0',
      max_oracle_age_sec: 21600,
      oracle: { rate: UST1_RATE_SCALE.toString(), last_update_sec: 1, paused: false },
    })
    const res = await getUst1EffectiveSwap()
    expect(queryContract).toHaveBeenCalledWith(WINDOW, { effective_swap: {} })
    expect(res.fee_bps).toBe(100)
    expect(res.oracle.paused).toBe(false)
  })

  it('deposit sends CW20 Send with deposit hook to window (not router)', async () => {
    vi.mocked(executeTerraContract).mockResolvedValue('txdep')
    await depositVfdusdForUst1(WALLET, '1000000')
    expect(executeTerraContract).toHaveBeenCalledWith(
      WALLET,
      VFDUSD,
      expect.objectContaining({
        send: expect.objectContaining({
          contract: WINDOW,
          amount: '1000000',
        }),
      })
    )
    const msg = (vi.mocked(executeTerraContract).mock.calls[0][2] as { send: { msg: string } }).send.msg
    expect(JSON.parse(atob(msg))).toEqual({ deposit: {} })
  })

  it('withdraw sends CW20 Send with min_vfdusd_out haircut', async () => {
    vi.mocked(executeTerraContract).mockResolvedValue('txwd')
    await withdrawUst1ForVfdusd(WALLET, '1000000', undefined, {
      fee_bps: 100,
      per_tx_ust1_limit: '1000000000',
      rolling_24h_ust1_limit: '10000000000',
      paused: false,
      rolling_window_start_sec: 0,
      rolling_volume_ust1: '0',
      max_oracle_age_sec: 21600,
      oracle: { rate: UST1_RATE_SCALE.toString(), last_update_sec: 1, paused: false },
    })
    const inner = JSON.parse(
      atob((vi.mocked(executeTerraContract).mock.calls[0][2] as { send: { msg: string } }).send.msg)
    ) as { withdraw: { min_vfdusd_out: string } }
    // quoted 990000 after 1% fee; 1% slippage haircut → 980100
    expect(inner.withdraw.min_vfdusd_out).toBe('980100')
    expect(vi.mocked(executeTerraContract).mock.calls[0][1]).toBe(UST1)
  })

  it('executeUst1Window routes by direction and pay tokens are allowlisted', async () => {
    vi.mocked(executeTerraContract).mockResolvedValue('tx')
    const eff = {
      fee_bps: 100,
      per_tx_ust1_limit: '1000000000',
      rolling_24h_ust1_limit: '10000000000',
      paused: false,
      rolling_window_start_sec: 0,
      rolling_volume_ust1: '0',
      max_oracle_age_sec: 21600,
      oracle: { rate: UST1_RATE_SCALE.toString(), last_update_sec: 1, paused: false },
    }
    await executeUst1Window('deposit', WALLET, '1', eff)
    expect(vi.mocked(executeTerraContract).mock.calls[0][1]).toBe(VFDUSD)
    expect(payTokenForDirection('deposit')).toBe(VFDUSD)
    expect(payTokenForDirection('withdraw')).toBe(UST1)
  })
})
