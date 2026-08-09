/**
 * ust1-window LCD client (GitLab #506).
 *
 * Deposit / withdraw execute via CW20 `Send` to the window — never the AMM router.
 * Quotes and gates use on-chain `effective_swap` (+ client integer math from that rate).
 */

import { queryContract } from './queries'
import { executeTerraContract } from './transactions'
import { UST1_TOKEN_ADDRESS, UST1_WINDOW_CONTRACT_ADDRESS, VFDUSD_TOKEN_ADDRESS } from '@/utils/constants'
import { minVfdusdOutAfterSlippage, withdrawGrossUst1ToVfdusd } from '@/utils/ust1WindowMath'
import type { Ust1EffectiveSwapView, Ust1WindowDirection } from '@/utils/ust1WindowGates'

export type Ust1WindowConfigResponse = {
  governance: string
  oracle: string
  vfdusd_token: string
  cmm_treasury: string
  ust1_token: string
  fee_bps: number
  per_tx_ust1_limit: string
  rolling_24h_ust1_limit: string
  paused: boolean
  max_oracle_age_sec: number
}

export type Ust1EffectiveSwapResponse = Ust1EffectiveSwapView & {
  fee_chain_tax_bps?: number
  fee_cmm_protocol_bps?: number
  oracle: Ust1EffectiveSwapView['oracle'] & {
    utc_day_id?: number
    day_baseline_rate?: string
  }
}

function requireWindowAddress(): string {
  const addr = UST1_WINDOW_CONTRACT_ADDRESS.trim()
  if (!addr) {
    throw new Error(
      'UST1 window address is not configured. Set VITE_UST1_WINDOW_ADDRESS (columbus-5 production values in docs).'
    )
  }
  return addr
}

export async function getUst1WindowConfig(): Promise<Ust1WindowConfigResponse> {
  return queryContract<Ust1WindowConfigResponse>(requireWindowAddress(), { config: {} })
}

export async function getUst1EffectiveSwap(): Promise<Ust1EffectiveSwapResponse> {
  const res = await queryContract<Ust1EffectiveSwapResponse>(requireWindowAddress(), {
    effective_swap: {},
  })
  // Normalize optional oracle.paused (pre-circuit-breaker readers / LCD omit → false).
  return {
    ...res,
    oracle: {
      ...res.oracle,
      paused: res.oracle.paused === true,
    },
  }
}

function encodeHook(msg: Record<string, unknown>): string {
  return btoa(JSON.stringify(msg))
}

/** Deposit: CW20 Send vFDUSD → window with `{ deposit: {} }`. */
export async function depositVfdusdForUst1(walletAddress: string, amountVfdusdRaw: string): Promise<string> {
  const window = requireWindowAddress()
  const token = VFDUSD_TOKEN_ADDRESS.trim()
  if (!token) throw new Error('VITE_VFDUSD_TOKEN_ADDRESS is not configured.')
  return executeTerraContract(walletAddress, token, {
    send: {
      contract: window,
      amount: amountVfdusdRaw,
      msg: encodeHook({ deposit: {} }),
    },
  })
}

/**
 * Withdraw: CW20 Send UST1 → window with `{ withdraw: { min_vfdusd_out } }`.
 * When `minVfdusdOut` is omitted, derives from `effective` quote with default slippage haircut.
 */
export async function withdrawUst1ForVfdusd(
  walletAddress: string,
  amountUst1Raw: string,
  minVfdusdOut?: string,
  effective?: Ust1EffectiveSwapView
): Promise<string> {
  const window = requireWindowAddress()
  const token = UST1_TOKEN_ADDRESS.trim()
  if (!token) throw new Error('VITE_UST1_TOKEN_ADDRESS is not configured.')

  let minOut = minVfdusdOut
  if (minOut == null) {
    if (!effective) {
      throw new Error('Withdraw requires min_vfdusd_out or an effective_swap quote.')
    }
    const quoted = withdrawGrossUst1ToVfdusd(BigInt(amountUst1Raw), BigInt(effective.oracle.rate), effective.fee_bps)
    minOut = minVfdusdOutAfterSlippage(quoted).toString()
  }

  return executeTerraContract(walletAddress, token, {
    send: {
      contract: window,
      amount: amountUst1Raw,
      msg: encodeHook({ withdraw: { min_vfdusd_out: minOut } }),
    },
  })
}

export async function executeUst1Window(
  direction: Ust1WindowDirection,
  walletAddress: string,
  payRaw: string,
  effective: Ust1EffectiveSwapView
): Promise<string> {
  if (direction === 'deposit') return depositVfdusdForUst1(walletAddress, payRaw)
  return withdrawUst1ForVfdusd(walletAddress, payRaw, undefined, effective)
}

export function payTokenForDirection(direction: Ust1WindowDirection): string {
  return direction === 'deposit' ? VFDUSD_TOKEN_ADDRESS : UST1_TOKEN_ADDRESS
}

export function receiveSymbolForDirection(direction: Ust1WindowDirection): 'UST1' | 'vFDUSD' {
  return direction === 'deposit' ? 'UST1' : 'vFDUSD'
}

export function paySymbolForDirection(direction: Ust1WindowDirection): 'vFDUSD' | 'UST1' {
  return direction === 'deposit' ? 'vFDUSD' : 'UST1'
}
