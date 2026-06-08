import { expect, type Locator, type Page } from '@playwright/test'

const POOL_CARD_SELECTOR = '.shell-panel-strong'
const WRAP_POOL_PAIR_SYMBOL = 'LUNC-C'

const MISSING_WRAP_PAIR_MSG =
  'LUNC-C pool card not found after pool search and pagination; run scripts/e2e-seed-wrap-pairs.sh (GitLab #201, #340).'

const INDEXER_OUTAGE_MSG =
  'Pool list indexer outage — start indexer for wrap-pool E2E (GitLab #340).'

/** Escape user-provided symbol for safe use inside a RegExp. */
function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

/**
 * Locator for a pool card that exposes provide/withdraw controls and includes `symbol`
 * as a whole token (avoids fuzzy matches like ALUNC-C).
 */
export function poolCardBySymbol(page: Page, symbol: string): Locator {
  const token = escapeRegExp(symbol)
  return page
    .locator(POOL_CARD_SELECTOR)
    .filter({ has: page.getByRole('button', { name: /^Provide Liquidity$/i }) })
    .filter({ hasText: new RegExp(`\\b${token}\\b`) })
    .first()
}

async function assertPoolIndexerAvailable(page: Page): Promise<void> {
  const outage = page.getByTestId('pool-market-data-outage-banner')
  if (await outage.isVisible()) {
    throw new Error(INDEXER_OUTAGE_MSG)
  }
}

async function waitForPoolListSettled(page: Page): Promise<void> {
  await expect(async () => {
    await assertPoolIndexerAvailable(page)
    const loading = await page.locator('[aria-live="polite"] .animate-pulse').count()
    expect(loading).toBe(0)
    const cards = await page
      .locator(POOL_CARD_SELECTOR)
      .filter({ has: page.getByRole('button', { name: /^Provide Liquidity$/i }) })
      .count()
    const empty = await page.getByText(/No liquidity pools match your filters/i).isVisible()
    const routerEmpty = await page.getByText(/No pools on this page are in the factory router set/i).isVisible()
    expect(cards > 0 || empty || routerEmpty).toBe(true)
  }).toPass({ timeout: 30_000 })
}

async function submitPoolSearch(page: Page, query: string): Promise<void> {
  await page.locator('#pool-search').fill(query)
  await page.getByRole('search', { name: /Filter and sort pools/i }).getByRole('button', { name: /^Search$/i }).click()
  await waitForPoolListSettled(page)
}

async function waitForPoolCardVisible(pairCard: Locator, timeoutMs = 30_000): Promise<boolean> {
  try {
    await expect(pairCard).toBeVisible({ timeout: timeoutMs })
    return true
  } catch {
    return false
  }
}

async function findPoolCardOnCurrentPage(page: Page, symbol: string): Promise<Locator | null> {
  const pairCard = poolCardBySymbol(page, symbol)
  return (await waitForPoolCardVisible(pairCard, 5_000)) ? pairCard : null
}

async function paginateToPoolCard(page: Page, symbol: string): Promise<Locator | null> {
  const nextButton = page.getByRole('button', { name: /^Next$/i })
  for (let pageIdx = 0; pageIdx < 50; pageIdx += 1) {
    const found = await findPoolCardOnCurrentPage(page, symbol)
    if (found) return found
    if (!(await nextButton.isEnabled())) break
    await nextButton.click()
  }
  return null
}

export type GotoPoolCardOptions = {
  /** Indexer `q` override (e.g. pair contract address substring). */
  searchQuery?: string
  /** When true, navigate to `/pool` before searching. */
  goto?: boolean
}

/**
 * Locate a pool card by symbol using indexer search, with pagination fallback when
 * `PAGE_SIZE` (20) hides the pair on the default list (GitLab #340).
 */
export async function gotoPoolCardBySymbol(
  page: Page,
  symbol: string,
  opts?: GotoPoolCardOptions
): Promise<Locator> {
  if (opts?.goto) {
    await page.goto('/pool')
    await expect(page.getByRole('heading', { name: /Liquidity Pools/i })).toBeVisible({ timeout: 90_000 })
  }

  const searchQuery = opts?.searchQuery ?? symbol

  await submitPoolSearch(page, searchQuery)
  let pairCard = await findPoolCardOnCurrentPage(page, symbol)
  if (pairCard) return pairCard

  // Search may return multiple pages (rare); walk Next within the filtered result set.
  pairCard = await paginateToPoolCard(page, symbol)
  if (pairCard) return pairCard

  // Fallback: clear search and paginate the full list (local 25-pair deploy without search hit).
  await submitPoolSearch(page, '')
  pairCard = await paginateToPoolCard(page, symbol)
  if (pairCard) return pairCard

  await assertPoolIndexerAvailable(page)
  expect(false, MISSING_WRAP_PAIR_MSG).toBe(true)
  return poolCardBySymbol(page, symbol)
}

/** Default LUNC-C wrap-pool card (seeded LUNC-C/EMBER pair). */
export async function gotoWrapPoolLuncCard(page: Page): Promise<Locator> {
  const tokenHint = process.env.VITE_LUNC_C_TOKEN_ADDRESS?.trim()
  return gotoPoolCardBySymbol(page, WRAP_POOL_PAIR_SYMBOL, {
    searchQuery: tokenHint || WRAP_POOL_PAIR_SYMBOL,
  })
}
