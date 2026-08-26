import { describe, expect, it } from 'vitest'
import { COMMUNITY_TAX_SKUS } from './communityTaxSku'
import {
  MIGRATE_WHY_EXAMPLES,
  MIGRATE_WHY_HEADLINE,
  migrateUnlockFeatureCount,
  migrateWhyHeadline,
} from './communityTaxMigrateCopy'

describe('communityTaxMigrateCopy (#670)', () => {
  it('X is 1 + post-create SKUs and excludes mint_control', () => {
    const postCreate = COMMUNITY_TAX_SKUS.filter((s) => !s.createOnly)
    expect(COMMUNITY_TAX_SKUS.find((s) => s.id === 'mint_control')?.createOnly).toBe(true)
    expect(migrateUnlockFeatureCount()).toBe(1 + postCreate.length)
    expect(migrateUnlockFeatureCount()).toBe(7)
    expect(postCreate.map((s) => s.id)).not.toContain('mint_control')
  })

  it('headline uses the derived count, not a hardcoded 8', () => {
    const n = migrateUnlockFeatureCount()
    expect(MIGRATE_WHY_HEADLINE).toBe(`Unlock ${n} features for your token on CL8Y Dex by migrating today`)
    expect(migrateWhyHeadline()).toBe(MIGRATE_WHY_HEADLINE)
    expect(MIGRATE_WHY_HEADLINE).not.toMatch(/Unlock 8 /)
  })

  it('examples name high-value retail gains and never Minting', () => {
    expect(MIGRATE_WHY_EXAMPLES).toMatch(/buy and sell tax/i)
    expect(MIGRATE_WHY_EXAMPLES).toMatch(/listed CL8Y pair/i)
    expect(MIGRATE_WHY_EXAMPLES).toContain('Auto liquidity')
    expect(MIGRATE_WHY_EXAMPLES).toContain('Launch guards')
    expect(MIGRATE_WHY_EXAMPLES).toContain('Extra exemptions')
    expect(MIGRATE_WHY_EXAMPLES).toContain('Split treasury')
    expect(MIGRATE_WHY_EXAMPLES).not.toMatch(/Minting/)
    expect(MIGRATE_WHY_EXAMPLES).not.toMatch(/mint_control/)
    expect(MIGRATE_WHY_EXAMPLES).not.toMatch(/enabled/i)
    expect(MIGRATE_WHY_EXAMPLES).not.toMatch(/turned on/i)
    expect(MIGRATE_WHY_EXAMPLES).not.toMatch(/50 UST1/)
    expect(MIGRATE_WHY_EXAMPLES).not.toMatch(/enable_feature/)
    expect(MIGRATE_WHY_EXAMPLES).not.toMatch(/VITE_/)
    expect(MIGRATE_WHY_HEADLINE).not.toMatch(/Minting/)
  })
})
