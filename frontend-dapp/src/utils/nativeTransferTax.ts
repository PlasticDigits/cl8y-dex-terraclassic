import { TERRA_LCD_URL } from '@/utils/constants'

/** Classic burn-tax params from LCD (GitLab #342). */
export type NativeTransferTaxParams = {
  rate: string
  capUluna: bigint
}

const DEFAULT_TAX_RATE = '0.005'
const DEFAULT_TAX_CAP_ULUNA = 1_000_000_000_000_000n

const cachedTaxByDenom = new Map<string, { at: number; params: NativeTransferTaxParams }>()
const CACHE_MS = 60_000

/**
 * Tax deducted on native transfers (burn tax). Mirrors classic Terraswap:
 * `min(amount - amount/(1+rate), cap)` as the tax component; net = gross - tax.
 */
export function netUlunaAfterTransferTax(grossUluna: bigint, params: NativeTransferTaxParams): bigint {
  if (grossUluna <= 0n) return 0n
  const tax = computeNativeTransferTaxUluna(grossUluna, params)
  return grossUluna - tax
}

/** Classic: tax = min(gross - gross/(1+rate), cap) — see Terraswap `compute_tax`. */
export function computeNativeTransferTaxUluna(grossUluna: bigint, params: NativeTransferTaxParams): bigint {
  if (grossUluna <= 0n) return 0n
  const rateScaled = parseTaxRateToScaled(params.rate)
  if (rateScaled.num <= 0n) return 0n
  const onePlusRateDen = rateScaled.den + rateScaled.num
  const afterTax = (grossUluna * rateScaled.den) / onePlusRateDen
  const taxFromRate = grossUluna - afterTax
  const capped = taxFromRate > params.capUluna ? params.capUluna : taxFromRate
  return capped
}

function parseTaxRateToScaled(rate: string): { num: bigint; den: bigint } {
  const trimmed = rate.trim()
  const m = /^(\d+)(?:\.(\d+))?$/.exec(trimmed)
  if (!m) return { num: 0n, den: 1n }
  const intPart = m[1] || '0'
  let frac = m[2] ?? ''
  frac = frac.replace(/0+$/, '')
  const den = 10n ** BigInt(frac.length || 0)
  const num = BigInt(intPart) * den + (frac ? BigInt(frac) : 0n)
  return { num, den }
}

/** Fetch tax rate + cap for `uluna` (cached 60s). */
export async function fetchNativeTransferTaxParams(denom = 'uluna'): Promise<NativeTransferTaxParams> {
  const now = Date.now()
  const cached = cachedTaxByDenom.get(denom)
  if (cached && now - cached.at < CACHE_MS) return cached.params

  let rate = DEFAULT_TAX_RATE
  let capUluna = DEFAULT_TAX_CAP_ULUNA
  try {
    const rateResp = await fetch(`${TERRA_LCD_URL}/terra/tax/v1beta1/tax_rate`)
    if (rateResp.ok) {
      const body = (await rateResp.json()) as { tax_rate?: string }
      if (body.tax_rate?.trim()) rate = body.tax_rate.trim()
    }
    const capResp = await fetch(`${TERRA_LCD_URL}/terra/tax/v1beta1/tax_caps/${denom}`)
    if (capResp.ok) {
      const body = (await capResp.json()) as { tax_cap?: string }
      if (body.tax_cap?.trim()) capUluna = BigInt(body.tax_cap.trim())
    }
  } catch {
    // LCD unavailable — use defaults (LocalTerra / Classic ~0.5%)
  }

  const params = { rate, capUluna }
  cachedTaxByDenom.set(denom, { at: now, params })
  return params
}

export async function netUlunaAfterTransferTaxAsync(grossUluna: bigint, denom = 'uluna'): Promise<bigint> {
  const params = await fetchNativeTransferTaxParams(denom)
  return netUlunaAfterTransferTax(grossUluna, params)
}

/**
 * Smallest gross uluna whose post-tax net is ≥ `targetNet`.
 * Used when auto-filling a native-wrap provide side from a net-ratio counterpart.
 */
export function grossUlunaForTargetNet(targetNet: bigint, params: NativeTransferTaxParams): bigint {
  if (targetNet <= 0n) return 0n

  let lo = targetNet
  let hi = targetNet + 1n
  while (netUlunaAfterTransferTax(hi, params) < targetNet) {
    lo = hi
    hi *= 2n
  }

  while (lo < hi) {
    const mid = (lo + hi) / 2n
    if (netUlunaAfterTransferTax(mid, params) >= targetNet) {
      hi = mid
    } else {
      lo = mid + 1n
    }
  }
  return lo
}
