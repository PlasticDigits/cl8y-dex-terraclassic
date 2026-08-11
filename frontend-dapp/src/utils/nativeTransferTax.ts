import { TERRA_LCD_URL } from '@/utils/constants'

/**
 * Classic burn-tax params from LCD (GitLab #342 / #512).
 * Queried from `/terra/tax/v1beta1/params` (`burn_tax_rate`).
 */
export type NativeTransferTaxParams = {
  rate: string
  capUluna: bigint
}

/** Columbus-5 post–Prop #12223 default when LCD params are unavailable. */
const DEFAULT_TAX_RATE = '0.015'
const DEFAULT_TAX_CAP_ULUNA = 1_000_000_000_000_000n

const cachedTaxByDenom = new Map<string, { at: number; params: NativeTransferTaxParams }>()
const CACHE_MS = 60_000

/**
 * Tax deducted on native `BankMsg::Send` (burn tax). Classic ComputeTax:
 * `min(floor(amount × burn_tax_rate), cap)` — recipient receives `amount − tax`.
 *
 * Confirmed on columbus-5 unwrap InstantWithdraw (GitLab #512): 9_800 × 0.015 = 147.
 * Not the Terraswap stable-tax inverse formula (`amount − amount/(1+rate)`).
 *
 * `MsgExecuteContract` with funds (user → contract wrap_deposit) is **not** taxed —
 * do not apply this to wrap mint quotes (#512 wrap display).
 */
export function netUlunaAfterTransferTax(grossUluna: bigint, params: NativeTransferTaxParams): bigint {
  if (grossUluna <= 0n) return 0n
  const tax = computeNativeTransferTaxUluna(grossUluna, params)
  return grossUluna - tax
}

/** Classic burn tax: `min(floor(gross × rate), cap)`. */
export function computeNativeTransferTaxUluna(grossUluna: bigint, params: NativeTransferTaxParams): bigint {
  if (grossUluna <= 0n) return 0n
  const rateScaled = parseTaxRateToScaled(params.rate)
  if (rateScaled.num <= 0n) return 0n
  // floor(gross * num / den)
  const taxFromRate = (grossUluna * rateScaled.num) / rateScaled.den
  return taxFromRate > params.capUluna ? params.capUluna : taxFromRate
}

function parseTaxRateToScaled(rate: string): { num: bigint; den: bigint } {
  const trimmed = rate.trim()
  const m = /^(\d+)(?:\.(\d+))?$/.exec(trimmed)
  if (!m) return { num: 0n, den: 1n }
  const intPart = m[1] || '0'
  let frac = m[2] ?? ''
  frac = frac.replace(/0+$/, '')
  if (!frac) {
    return { num: BigInt(intPart), den: 1n }
  }
  const den = 10n ** BigInt(frac.length)
  const num = BigInt(intPart) * den + BigInt(frac)
  return { num, den }
}

/** Fetch burn tax rate + cap for `uluna`/`uusd` (cached 60s). */
export async function fetchNativeTransferTaxParams(denom = 'uluna'): Promise<NativeTransferTaxParams> {
  const now = Date.now()
  const cached = cachedTaxByDenom.get(denom)
  if (cached && now - cached.at < CACHE_MS) return cached.params

  let rate = DEFAULT_TAX_RATE
  let capUluna = DEFAULT_TAX_CAP_ULUNA
  try {
    const paramsResp = await fetch(`${TERRA_LCD_URL}/terra/tax/v1beta1/params`)
    if (paramsResp.ok) {
      const body = (await paramsResp.json()) as { params?: { burn_tax_rate?: string }; burn_tax_rate?: string }
      const burn = body.params?.burn_tax_rate?.trim() || body.burn_tax_rate?.trim()
      if (burn) rate = burn
    } else {
      // Legacy fallback (often unimplemented on columbus-5 public LCDs).
      const rateResp = await fetch(`${TERRA_LCD_URL}/terra/tax/v1beta1/tax_rate`)
      if (rateResp.ok) {
        const body = (await rateResp.json()) as { tax_rate?: string }
        if (body.tax_rate?.trim()) rate = body.tax_rate.trim()
      }
    }

    const capResp = await fetch(`${TERRA_LCD_URL}/terra/treasury/v1beta1/tax_caps/${denom}`)
    if (capResp.ok) {
      const body = (await capResp.json()) as { tax_cap?: string }
      if (body.tax_cap?.trim()) capUluna = BigInt(body.tax_cap.trim())
    }
  } catch {
    // LCD unavailable — use columbus-5 defaults
  }

  const params = { rate, capUluna }
  cachedTaxByDenom.set(denom, { at: now, params })
  return params
}

/** Test helper — clear in-memory tax cache. */
export function clearNativeTransferTaxCache(): void {
  cachedTaxByDenom.clear()
}

export async function netUlunaAfterTransferTaxAsync(grossUluna: bigint, denom = 'uluna'): Promise<bigint> {
  const params = await fetchNativeTransferTaxParams(denom)
  return netUlunaAfterTransferTax(grossUluna, params)
}

/**
 * Smallest gross uluna whose post-tax net is ≥ `targetNet`.
 * Used when inverting a taxed BankMsg::Send net → pre-tax send amount
 * (e.g. future unwrap gross-up; pool paths that still model taxed native legs).
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

/** Format burn tax rate for short UI fee notes (e.g. `1.5%`). */
export function formatBurnTaxPercentLabel(rate: string): string | null {
  const scaled = parseTaxRateToScaled(rate)
  if (scaled.num <= 0n) return null
  // percent = num/den * 100
  const pctNum = scaled.num * 100n
  const whole = pctNum / scaled.den
  const rem = pctNum % scaled.den
  if (rem === 0n) return `${whole.toString()}%`
  // up to 4 decimal places of percent, trim trailing zeros
  const scale = 10000n
  const frac = ((rem * scale) / scaled.den).toString().padStart(4, '0').replace(/0+$/, '')
  return frac ? `${whole.toString()}.${frac}%` : `${whole.toString()}%`
}
