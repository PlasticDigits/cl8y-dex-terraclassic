import { describe, expect, it } from 'vitest'
import { ALPHA_COLUMBUS5_ADDR, buildAdoptMigrateMsg, classifyMigrateSource } from './communityTaxMigrate'

const ADMIN = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
const TOKEN = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'

describe('communityTaxMigrate (#626)', () => {
  it('P3: listed 10184 + admin wallet → go, no invoice fields', () => {
    const v = classifyMigrateSource({
      chainId: 'columbus-5',
      codeId: 10184,
      taxCodeId: 11619,
      whitelisted: true,
      hasTaxMap: false,
      wasmAdmin: ADMIN,
      connectedWallet: ADMIN,
      tokenAddr: TOKEN,
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

  it('P4: 8266 listed honest is go', () => {
    const v = classifyMigrateSource({
      chainId: 'columbus-5',
      codeId: 8266,
      taxCodeId: 11619,
      whitelisted: true,
      hasTaxMap: false,
      wasmAdmin: ADMIN,
      connectedWallet: ADMIN,
      tokenAddr: TOKEN,
    })
    expect(v.kind).toBe('go')
  })

  it('P6: ALPHA / 8654 wipe path is go without factory whitelist', () => {
    const v = classifyMigrateSource({
      chainId: 'columbus-5',
      codeId: 8654,
      taxCodeId: 11619,
      whitelisted: false,
      hasTaxMap: true,
      wasmAdmin: ADMIN,
      connectedWallet: ADMIN,
      tokenAddr: ALPHA_COLUMBUS5_ADDR,
    })
    expect(v.kind).toBe('go_alpha')
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
      tokenAddr: ALPHA_COLUMBUS5_ADDR,
    })
    expect(msg.adopt.buy_bps).toBe(450)
    expect(msg.adopt.sell_bps).toBe(100)
  })

  it('P2: already 11619 is CMM-only', () => {
    const v = classifyMigrateSource({
      chainId: 'columbus-5',
      codeId: 11619,
      taxCodeId: 11619,
      whitelisted: true,
      hasTaxMap: false,
      wasmAdmin: ADMIN,
      connectedWallet: ADMIN,
      tokenAddr: TOKEN,
    })
    expect(v.kind).toBe('already_tax')
    expect(v.canSubmit).toBe(false)
  })

  it('A13: unlisted source refuses', () => {
    const v = classifyMigrateSource({
      chainId: 'columbus-5',
      codeId: 99999,
      taxCodeId: 11619,
      whitelisted: false,
      hasTaxMap: false,
      wasmAdmin: ADMIN,
      connectedWallet: ADMIN,
      tokenAddr: TOKEN,
    })
    expect(v.kind).toBe('unlisted')
  })

  it('A10: non-admin cannot submit', () => {
    const v = classifyMigrateSource({
      chainId: 'columbus-5',
      codeId: 6036,
      taxCodeId: 11619,
      whitelisted: true,
      hasTaxMap: false,
      wasmAdmin: ADMIN,
      connectedWallet: TOKEN,
      tokenAddr: TOKEN,
    })
    expect(v.kind).toBe('not_admin')
    expect(v.canSubmit).toBe(false)
  })

  it('LocalTerra whitelisted analogue is go', () => {
    const v = classifyMigrateSource({
      chainId: 'localterra',
      codeId: 42,
      taxCodeId: 99,
      whitelisted: true,
      hasTaxMap: false,
      wasmAdmin: ADMIN,
      connectedWallet: ADMIN,
      tokenAddr: TOKEN,
    })
    expect(v.kind).toBe('go')
  })
})
