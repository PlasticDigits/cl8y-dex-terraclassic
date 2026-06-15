import { describe, expect, it } from 'vitest'
import {
  getTerraAddressInputError,
  hasTerraAddressFormat,
  INVALID_TERRA_ADDRESS_CHECKSUM_MSG,
  INVALID_TERRA_ADDRESS_FORMAT_MSG,
  isValidTerraBech32Address,
} from '../terraAddressValidation'

const VALID = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const VALID_LONG = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'

describe('terraAddressValidation', () => {
  it('accepts valid terra bech32 addresses', () => {
    expect(hasTerraAddressFormat(VALID)).toBe(true)
    expect(isValidTerraBech32Address(VALID)).toBe(true)
    expect(getTerraAddressInputError(VALID)).toBeNull()
    expect(isValidTerraBech32Address(VALID_LONG)).toBe(true)
  })

  it('rejects obvious format failures', () => {
    expect(getTerraAddressInputError('')).toBeNull()
    expect(getTerraAddressInputError('cosmos1abcdef')).toBe(INVALID_TERRA_ADDRESS_FORMAT_MSG)
    expect(getTerraAddressInputError('terra1')).toBe(INVALID_TERRA_ADDRESS_FORMAT_MSG)
    expect(getTerraAddressInputError('TERRA1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v')).toBe(
      INVALID_TERRA_ADDRESS_FORMAT_MSG
    )
  })

  it('rejects structurally plausible terra1 addresses with invalid checksum (GitLab #382)', () => {
    const corrupted = `${VALID.slice(0, -3)}289`
    expect(hasTerraAddressFormat(corrupted)).toBe(true)
    expect(isValidTerraBech32Address(corrupted)).toBe(false)
    expect(getTerraAddressInputError(corrupted)).toBe(INVALID_TERRA_ADDRESS_CHECKSUM_MSG)
  })
})
