import { queryContract, queryContractRaw } from './queries'
import {
  pairDiscountRegistryRawKeyB64,
  decodeCwStoragePlusOptionalAddr,
  parseGetDiscountRegistryResponse,
} from '@/utils/pairDiscountRegistry'

/**
 * Pair `DISCOUNT_REGISTRY` address, or `null` when unset.
 *
 * Live pair wasm (1.13.0) has no `GetDiscountRegistry` smart query; the storage
 * key `discount_registry` is readable via LCD raw state (#537). Smart query is
 * a fallback for future wasm / LCDs that 403 raw reads.
 */
export async function getPairDiscountRegistry(pairAddress: string): Promise<string | null> {
  const addr = pairAddress.trim()
  if (!addr.startsWith('terra1')) {
    throw new Error('Pair address is required')
  }
  try {
    const raw = await queryContractRaw(addr, pairDiscountRegistryRawKeyB64())
    return decodeCwStoragePlusOptionalAddr(raw)
  } catch (rawErr) {
    try {
      const resp = await queryContract<unknown>(addr, { get_discount_registry: {} })
      return parseGetDiscountRegistryResponse(resp)
    } catch {
      throw rawErr
    }
  }
}
