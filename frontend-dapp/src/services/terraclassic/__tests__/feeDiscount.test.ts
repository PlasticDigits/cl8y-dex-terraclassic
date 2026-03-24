import { describe, it, expect, vi, beforeEach } from 'vitest'

vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn(),
}))

vi.mock('@/services/terraclassic/transactions', () => ({
  executeTerraContract: vi.fn(),
}))

vi.mock('@/utils/constants', () => ({
  FACTORY_CONTRACT_ADDRESS: 'terra1factory',
  FEE_DISCOUNT_CONTRACT_ADDRESS: 'terra1feediscount',
  TERRA_LCD_URL: 'http://localhost:1317',
}))

import { queryContract } from '@/services/terraclassic/queries'
import { executeTerraContract } from '@/services/terraclassic/transactions'
import { getTraderDiscount, getTiers, getTier, getRegistration, register, deregister } from '../feeDiscount'
import type { DiscountResponse, RegistrationResponse, Tier, TierEntry } from '@/types'

const mockedQuery = vi.mocked(queryContract)
const mockedExecute = vi.mocked(executeTerraContract)

const FEE_CONTRACT = 'terra1feediscount'
const WALLET = 'terra1wallet'
const TRADER = 'terra1trader'

beforeEach(() => {
  vi.clearAllMocks()
})

describe('getTraderDiscount', () => {
  it('queries discount with trader as both trader and sender when senderAddr omitted', async () => {
    const discount: DiscountResponse = {
      discount_bps: 50,
      needs_deregister: false,
      registration_epoch: null,
    }
    mockedQuery.mockResolvedValueOnce(discount)

    const result = await getTraderDiscount(TRADER)

    expect(mockedQuery).toHaveBeenCalledWith(FEE_CONTRACT, {
      get_discount: { trader: TRADER, sender: TRADER },
    })
    expect(result).toEqual(discount)
  })

  it('queries discount with an explicit sender address', async () => {
    const discount: DiscountResponse = {
      discount_bps: 100,
      needs_deregister: true,
      registration_epoch: 0,
    }
    mockedQuery.mockResolvedValueOnce(discount)

    const senderAddr = 'terra1sender'
    const result = await getTraderDiscount(TRADER, senderAddr)

    expect(mockedQuery).toHaveBeenCalledWith(FEE_CONTRACT, {
      get_discount: { trader: TRADER, sender: senderAddr },
    })
    expect(result).toEqual(discount)
  })
})

describe('getTiers', () => {
  it('returns tier entries from query response', async () => {
    const tiers: TierEntry[] = [
      {
        tier_id: 1,
        tier: {
          min_cl8y_balance: '1000000',
          discount_bps: 25,
          governance_only: false,
        },
      },
      {
        tier_id: 2,
        tier: {
          min_cl8y_balance: '10000000',
          discount_bps: 50,
          governance_only: false,
        },
      },
    ]
    mockedQuery.mockResolvedValueOnce({ tiers })

    const result = await getTiers()

    expect(mockedQuery).toHaveBeenCalledWith(FEE_CONTRACT, { get_tiers: {} })
    expect(result).toEqual(tiers)
  })

  it('returns empty array when no tiers exist', async () => {
    mockedQuery.mockResolvedValueOnce({ tiers: [] })

    const result = await getTiers()

    expect(result).toEqual([])
  })
})

describe('getTier', () => {
  it('queries a specific tier by ID', async () => {
    const tier: Tier = {
      min_cl8y_balance: '5000000',
      discount_bps: 75,
      governance_only: true,
    }
    const response = { tier_id: 3, tier }
    mockedQuery.mockResolvedValueOnce(response)

    const result = await getTier(3)

    expect(mockedQuery).toHaveBeenCalledWith(FEE_CONTRACT, {
      get_tier: { tier_id: 3 },
    })
    expect(result).toEqual(response)
  })
})

describe('getRegistration', () => {
  it('queries registration status for a trader', async () => {
    const registration: RegistrationResponse = {
      registered: true,
      tier_id: 2,
      tier: {
        min_cl8y_balance: '10000000',
        discount_bps: 50,
        governance_only: false,
      },
    }
    mockedQuery.mockResolvedValueOnce(registration)

    const result = await getRegistration(TRADER)

    expect(mockedQuery).toHaveBeenCalledWith(FEE_CONTRACT, {
      get_registration: { trader: TRADER },
    })
    expect(result).toEqual(registration)
  })

  it('returns unregistered status', async () => {
    const registration: RegistrationResponse = {
      registered: false,
      tier_id: null,
      tier: null,
    }
    mockedQuery.mockResolvedValueOnce(registration)

    const result = await getRegistration(TRADER)

    expect(result.registered).toBe(false)
    expect(result.tier_id).toBeNull()
    expect(result.tier).toBeNull()
  })
})

describe('register', () => {
  it('executes register with the correct tier ID', async () => {
    mockedExecute.mockResolvedValueOnce('txhash_register')

    const result = await register(WALLET, 2)

    expect(result).toBe('txhash_register')
    expect(mockedExecute).toHaveBeenCalledWith(WALLET, FEE_CONTRACT, {
      register: { tier_id: 2 },
    })
  })
})

describe('deregister', () => {
  it('executes deregister with no additional args', async () => {
    mockedExecute.mockResolvedValueOnce('txhash_deregister')

    const result = await deregister(WALLET)

    expect(result).toBe('txhash_deregister')
    expect(mockedExecute).toHaveBeenCalledWith(WALLET, FEE_CONTRACT, {
      deregister: {},
    })
  })
})
