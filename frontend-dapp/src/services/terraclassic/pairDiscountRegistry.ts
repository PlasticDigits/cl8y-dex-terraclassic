import { queryContract, queryContractRaw } from './queries'
import {
  pairDiscountRegistryRawKeyB64,
  decodeCwStoragePlusOptionalAddr,
  parseGetDiscountRegistryResponse,
} from '@/utils/pairDiscountRegistry'

/**
 * Pair `DISCOUNT_REGISTRY` address, or `null` when unset.
 *
 * Pair wasm **1.14.0+** implements `GetDiscountRegistry`. Prefer that smart
 * query (GitLab #538). LCD raw key `discount_registry` remains the fallback
 * for 1.13.x wasm or LCDs that reject the query (#537). Probe failure is
 * fail-closed (callers treat as unwired / full fee chrome).
 */
export async function getPairDiscountRegistry(pairAddress: string): Promise<string | null> {
  const addr = pairAddress.trim()
  if (!addr.startsWith('terra1')) {
    throw new Error('Pair address is required')
  }
  try {
    const resp = await queryContract<unknown>(addr, { get_discount_registry: {} })
    return parseGetDiscountRegistryResponse(resp)
  } catch (smartErr) {
    try {
      const raw = await queryContractRaw(addr, pairDiscountRegistryRawKeyB64())
      return decodeCwStoragePlusOptionalAddr(raw)
    } catch {
      throw smartErr
    }
  }
}
