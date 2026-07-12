import { describe, it, expect, vi, beforeEach } from 'vitest'

vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn(),
}))

vi.mock('@/services/terraclassic/transactions', () => ({
  executeTerraContract: vi.fn(),
}))

vi.mock('@/utils/constants', () => ({
  FAUCET_CONTRACT_ADDRESS: 'terra1faucet000000000000000000000000000001',
}))

import { queryContract } from '@/services/terraclassic/queries'
import { executeTerraContract } from '@/services/terraclassic/transactions'
import { drip, getFaucetConfig, getFaucetCooldown } from '../faucet'
import type { FaucetConfigResponse, FaucetCooldownResponse } from '../faucet'

const mockedQuery = vi.mocked(queryContract)
const mockedExecute = vi.mocked(executeTerraContract)

const FAUCET = 'terra1faucet000000000000000000000000000001'
const WALLET = 'terra1wallet00000000000000000000000000001'
const TOKEN = 'terra1ember00000000000000000000000000000001'

beforeEach(() => {
  vi.clearAllMocks()
})

describe('getFaucetConfig', () => {
  it('queries config from the faucet contract', async () => {
    const config: FaucetConfigResponse = {
      admin: 'terra1admin',
      drip_amount: '100000000',
      cooldown_seconds: 300,
      paused: false,
      allowed_tokens: [TOKEN],
    }
    mockedQuery.mockResolvedValueOnce(config)

    const result = await getFaucetConfig()

    expect(mockedQuery).toHaveBeenCalledWith(FAUCET, { config: {} })
    expect(result).toEqual(config)
  })
})

describe('getFaucetCooldown', () => {
  it('queries cooldown for a wallet address', async () => {
    const cooldown: FaucetCooldownResponse = {
      can_claim: false,
      seconds_remaining: 120,
      last_claim_at: '1700000000000000000',
      paused: false,
    }
    mockedQuery.mockResolvedValueOnce(cooldown)

    const result = await getFaucetCooldown(WALLET)

    expect(mockedQuery).toHaveBeenCalledWith(FAUCET, { cooldown: { address: WALLET } })
    expect(result).toEqual(cooldown)
  })
})

describe('drip', () => {
  it('executes drip with the selected token address', async () => {
    mockedExecute.mockResolvedValueOnce('txhash_drip')

    const result = await drip(WALLET, TOKEN)

    expect(result).toBe('txhash_drip')
    expect(mockedExecute).toHaveBeenCalledWith(WALLET, FAUCET, {
      drip: { token: TOKEN },
    })
  })
})
