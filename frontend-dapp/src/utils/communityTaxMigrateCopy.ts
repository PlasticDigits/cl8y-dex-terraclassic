/**
 * Retail why-copy for `/token/migrate` (GitLab #670).
 *
 * Unlock {X} is **template access**, not auto-enable. X = built-in listed-pair
 * buy/sell tax + SKUs with `createOnly === false`. Minting stays create-only
 * (**C593-5**) and is never advertised here (**M670-2**).
 */

import { COMMUNITY_TAX_SKUS, type CommunityTaxSkuId } from './communityTaxSku'

/** Built-in listed-pair buy/sell tax is not a SKU; it still counts toward X. */
const BUILTIN_LISTED_PAIR_TAX_FEATURES = 1

function skuLabel(id: CommunityTaxSkuId): string {
  return COMMUNITY_TAX_SKUS.find((s) => s.id === id)?.label ?? id
}

/**
 * Canonical headline count. Do not hardcode `7` at call sites — a future
 * post-create SKU must bump X with the catalog (**M670-1**).
 */
export function migrateUnlockFeatureCount(): number {
  return BUILTIN_LISTED_PAIR_TAX_FEATURES + COMMUNITY_TAX_SKUS.filter((s) => !s.createOnly).length
}

export function migrateWhyHeadline(count = migrateUnlockFeatureCount()): string {
  return `Unlock ${count} features for your token on CL8Y Dex by migrating today`
}

export const MIGRATE_WHY_HEADLINE = migrateWhyHeadline()

/**
 * Three-to-five high-value examples. Retail labels from the SKU catalog.
 * Does not list Minting or every SKU. Does not claim the migrate tx enables them.
 */
export const MIGRATE_WHY_EXAMPLES = [
  'The address does not change.',
  `You can use buy and sell tax on every listed CL8Y pair, ${skuLabel('auto_v2_lp')} into a factory pool, ${skuLabel('launch_guards')} (max wallet, cooldown, trading on/off), ${skuLabel('exemption_directory')}, and ${skuLabel('split_router')}.`,
  'Turn features on later from Manage.',
].join(' ')
