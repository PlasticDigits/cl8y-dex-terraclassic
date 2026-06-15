import { fromBech32 } from '@cosmjs/encoding'
import { isValidTerraAddress } from '@/utils/constants'

/** Inline copy when `terra1…` charset/length fails ([GitLab #382](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/382)). */
export const INVALID_TERRA_ADDRESS_FORMAT_MSG = 'Invalid Terra address format'

/** Inline copy when format passes but bech32 checksum fails ([#382](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/382)). */
export const INVALID_TERRA_ADDRESS_CHECKSUM_MSG =
  'Invalid address: checksum does not match. Please check and re-enter the token address.'

/** Same retail copy as inline validation — used when a tx error slips through. */
export const INVALID_TERRA_ADDRESS_CHECKSUM_TX_MSG = INVALID_TERRA_ADDRESS_CHECKSUM_MSG

/** `terra1…` charset + minimum length (no bech32 checksum). Used for trade deep links. */
export function hasTerraAddressFormat(addr: string): boolean {
  return isValidTerraAddress(addr)
}

/** Format + `terra` bech32 checksum — required before submitting token contract addresses. */
export function isValidTerraBech32Address(addr: string): boolean {
  return getTerraAddressInputError(addr) === null
}

/** `null` when valid; otherwise a short inline error for controlled address inputs. */
export function getTerraAddressInputError(addr: string): string | null {
  if (!addr) return null
  if (!hasTerraAddressFormat(addr)) return INVALID_TERRA_ADDRESS_FORMAT_MSG
  try {
    const { prefix } = fromBech32(addr)
    if (prefix !== 'terra') return INVALID_TERRA_ADDRESS_FORMAT_MSG
    return null
  } catch {
    return INVALID_TERRA_ADDRESS_CHECKSUM_MSG
  }
}
