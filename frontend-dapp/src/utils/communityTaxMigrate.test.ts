import { describe, expect, it } from 'vitest'
import {
  buildAdoptMigrateMsg,
  classifyMigrateSource,
  DEFAULT_COMMUNITY_MIGRATE_CODE_IDS,
  parseCommunityMigrateCodeIds,
} from './communityTaxMigrate'

const ADMIN = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
const TOKEN = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const LIST = DEFAULT_COMMUNITY_MIGRATE_CODE_IDS

describe('communityTaxMigrate (#626)', () => {
  it('parses env allowlist and keeps 8654 as a normal default id', () => {
    expect(parseCommunityMigrateCodeIds(undefined)).toEqual([6036, 10184, 8266, 8654])
    expect(parseCommunityMigrateCodeIds('6036,10184,8266,8654,99999')).toEqual([6036, 10184, 8266, 8654, 99999])
    expect(parseCommunityMigrateCodeIds('')).toEqual([6036, 10184, 8266, 8654])
  })

  it('P3: allowlisted 10184 + admin wallet → go, no invoice fields', () => {
    const v = classifyMigrateSource({
      chainId: 'columbus-5',
      codeId: 10184,
      taxCodeId: 11619,
      hasTaxMap: false,
      wasmAdmin: ADMIN,
      connectedWallet: ADMIN,
      allowedCodeIds: LIST,
    })
    expect(v.kind).toBe('go')
    expect(v.canSubmit).toBe(true)
    const msg = buildAdoptMigrateMsg({
      manager: ADMIN,
      treasury: ADMIN,
      factory: ADMIN,
      router: null,
      ust1: ADMIN,
      cmmTreasury: ADMIN,
      officialLauncher: ADMIN,
      sourceCodeId: 10184,
    })
    expect(msg.adopt.buy_bps).toBe(0)
    expect(msg.adopt).not.toHaveProperty('payee')
  })

  it('P4: 8266 allowlisted honest is go', () => {
    const v = classifyMigrateSource({
      chainId: 'columbus-5',
      codeId: 8266,
      taxCodeId: 11619,
      hasTaxMap: false,
      wasmAdmin: ADMIN,
      connectedWallet: ADMIN,
      allowedCodeIds: LIST,
    })
    expect(v.kind).toBe('go')
  })

  it('P6: 8654 is a normal allowlisted source (not a special case)', () => {
    const v = classifyMigrateSource({
      chainId: 'columbus-5',
      codeId: 8654,
      taxCodeId: 11619,
      factoryWhitelisted: false,
      hasTaxMap: true,
      wasmAdmin: ADMIN,
      connectedWallet: ADMIN,
      allowedCodeIds: LIST,
    })
    expect(v.kind).toBe('go')
    expect(v.canSubmit).toBe(true)
    const msg = buildAdoptMigrateMsg({
      manager: ADMIN,
      treasury: ADMIN,
      factory: ADMIN,
      router: null,
      ust1: ADMIN,
      cmmTreasury: ADMIN,
      officialLauncher: ADMIN,
      sourceCodeId: 8654,
      hasTaxMap: true,
    })
    expect(msg.adopt.buy_bps).toBe(450)
    expect(msg.adopt.sell_bps).toBe(100)
  })

  it('future allowlisted code id is go without factory listing', () => {
    const v = classifyMigrateSource({
      chainId: 'columbus-5',
      codeId: 77777,
      taxCodeId: 11619,
      factoryWhitelisted: false,
      hasTaxMap: false,
      wasmAdmin: ADMIN,
      connectedWallet: ADMIN,
      allowedCodeIds: [...LIST, 77777],
    })
    expect(v.kind).toBe('go')
    expect(v.canSubmit).toBe(true)
  })

  it('P2: already 11619 is CMM-only', () => {
    const v = classifyMigrateSource({
      chainId: 'columbus-5',
      codeId: 11619,
      taxCodeId: 11619,
      hasTaxMap: false,
      wasmAdmin: ADMIN,
      connectedWallet: ADMIN,
      allowedCodeIds: LIST,
    })
    expect(v.kind).toBe('already_tax')
    expect(v.canSubmit).toBe(false)
  })

  it('#627: columbus-5 code 3 is not a default migrate source', () => {
    expect(DEFAULT_COMMUNITY_MIGRATE_CODE_IDS).not.toContain(3)
    const v = classifyMigrateSource({
      chainId: 'columbus-5',
      codeId: 3,
      taxCodeId: 11619,
      factoryWhitelisted: true,
      hasTaxMap: false,
      wasmAdmin: ADMIN,
      connectedWallet: ADMIN,
      allowedCodeIds: LIST,
    })
    expect(v.kind).toBe('unlisted')
    expect(v.canSubmit).toBe(false)
  })

  it('A13: code id off the migrate allowlist refuses', () => {
    const v = classifyMigrateSource({
      chainId: 'columbus-5',
      codeId: 99999,
      taxCodeId: 11619,
      factoryWhitelisted: true,
      hasTaxMap: false,
      wasmAdmin: ADMIN,
      connectedWallet: ADMIN,
      allowedCodeIds: LIST,
    })
    expect(v.kind).toBe('unlisted')
    expect(v.canSubmit).toBe(false)
  })

  it('A10: non-admin cannot submit', () => {
    const v = classifyMigrateSource({
      chainId: 'columbus-5',
      codeId: 6036,
      taxCodeId: 11619,
      hasTaxMap: false,
      wasmAdmin: ADMIN,
      connectedWallet: TOKEN,
      allowedCodeIds: LIST,
    })
    expect(v.kind).toBe('not_admin')
    expect(v.canSubmit).toBe(false)
  })

  it('LocalTerra factory-whitelisted analogue is go when not on the env list', () => {
    const v = classifyMigrateSource({
      chainId: 'localterra',
      codeId: 42,
      taxCodeId: 99,
      factoryWhitelisted: true,
      hasTaxMap: false,
      wasmAdmin: ADMIN,
      connectedWallet: ADMIN,
      allowedCodeIds: LIST,
    })
    expect(v.kind).toBe('go')
  })
})
