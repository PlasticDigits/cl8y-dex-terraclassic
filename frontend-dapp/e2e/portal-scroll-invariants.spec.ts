import { expect, test, type Page } from '@playwright/test'
import { installScrollInvariantMarket, SCROLL_PAIR } from './helpers/scroll-invariants-fixture'

const GAP = 8
const TOLERANCE = 2
// Fixture identifiers reuse valid addresses; these synthetic markets only
// provide enough rows to verify overflow and never submit transactions.
const PAIRS = [
  SCROLL_PAIR.pair_address,
  'terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg',
  'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3',
  'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72',
  'terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3',
  'terra1yw4xvtc43me9scqfr2jr2gzvcxd3a9y4eq7gaukreugw2yd2f8tsrnr34u',
  'terra1yyca08xqdgvjz0psg56z67ejh9xms6l436u8y58m82npdqqhmmtqzjqhh0',
  'terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2',
].map((pair_address) => ({ ...SCROLL_PAIR, pair_address }))

async function geometry(page: Page) {
  return page.evaluate(() => {
    const trigger = document.querySelector<HTMLElement>('#trade-pair-select')!
    const anchor = trigger.closest('.token-select-root')!.getBoundingClientRect()
    const menu = document.querySelector<HTMLElement>('.token-select-dropdown')!
    const box = menu.getBoundingClientRect()
    const header = document.querySelector<HTMLElement>('.app-top-sticky')!.getBoundingClientRect()
    const nav = document.querySelector<HTMLElement>('.app-mobile-nav-shell')
    const navTop = nav && getComputedStyle(nav).display !== 'none' ? nav.getBoundingClientRect().top : innerHeight
    return {
      anchorTop: anchor.top,
      anchorBottom: anchor.bottom,
      anchorLeft: anchor.left,
      menuTop: box.top,
      menuBottom: box.bottom,
      menuLeft: box.left,
      menuRight: box.right,
      menuHeight: box.height,
      below: menu.style.top !== 'auto',
      headerBottom: header.bottom,
      navTop,
      width: innerWidth,
      portaled: menu.parentElement === document.body,
    }
  })
}

async function assertTracksTrigger(page: Page) {
  await expect(async () => {
    const g = await geometry(page)
    const distance = g.below ? g.menuTop - g.anchorBottom : g.anchorTop - g.menuBottom
    expect(Math.abs(distance - GAP)).toBeLessThanOrEqual(TOLERANCE)
    expect(Math.abs(g.menuLeft - g.anchorLeft)).toBeLessThanOrEqual(TOLERANCE)
    expect(g.portaled).toBe(true)
  }).toPass({ timeout: 5_000 })
}

async function assertInsideChrome(page: Page) {
  await expect(async () => {
    const g = await geometry(page)
    expect(g.menuTop).toBeGreaterThanOrEqual(g.headerBottom + GAP - TOLERANCE)
    expect(g.menuBottom).toBeLessThanOrEqual(g.navTop - GAP + TOLERANCE)
    expect(g.menuLeft).toBeGreaterThanOrEqual(GAP - TOLERANCE)
    expect(g.menuRight).toBeLessThanOrEqual(g.width - GAP + TOLERANCE)
    expect(g.menuHeight).toBeGreaterThan(40)
  }).toPass({ timeout: 5_000 })
}

for (const viewport of [
  { width: 1280, height: 720 },
  { width: 375, height: 667 },
]) {
  test.describe(`Portaled pair picker scroll invariants ${viewport.width}×${viewport.height} (#1316)`, () => {
    test.use({ viewport, hasTouch: viewport.width < 768 })

    test.beforeEach(async ({ page }) => {
      await installScrollInvariantMarket(page, { pairs: PAIRS })
      await page.goto(`/trade/${SCROLL_PAIR.pair_address}`, { waitUntil: 'domcontentloaded' })
      await expect(page.locator('#trade-pair-select')).toBeEnabled({ timeout: 90_000 })
    })

    test('document scroll follows the trigger and options remain selectable', async ({ page }) => {
      await page.locator('#trade-pair-select').click()
      await expect(page.getByRole('listbox', { name: 'Trading pair', exact: true })).toBeVisible()
      await assertTracksTrigger(page)
      const before = await geometry(page)
      await page.evaluate(() => window.scrollBy(0, 32))
      await expect(async () => {
        const after = await geometry(page)
        expect(before.anchorTop - after.anchorTop).toBeGreaterThan(20)
      }).toPass({ timeout: 5_000 })
      await assertTracksTrigger(page)
      await assertInsideChrome(page)
      await page.getByRole('listbox', { name: 'Trading pair', exact: true }).getByRole('option').last().click()
      await expect(page.locator('.token-select-dropdown')).toHaveCount(0)
      await expect(page).toHaveURL(new RegExp(`/trade/${PAIRS.at(-1)!.pair_address}$`))
    })

    test('captured inner scroll follows the trigger; menu scrolling keeps its position', async ({ page }) => {
      // The pair panel is normally document-scrolled. Give that real picker an
      // overflow ancestor to exercise the shared hook's capture scroll listener.
      await page.evaluate(() => {
        const panel = document.querySelector<HTMLElement>('[data-testid="trade-pair-select-panel"]')!
        panel.style.height = '190px'
        panel.style.overflowY = 'auto'
        panel.style.paddingTop = '70px'
        const spacer = document.createElement('div')
        spacer.style.height = '400px'
        panel.append(spacer)
      })
      await page.locator('#trade-pair-select').click()
      await expect(page.getByRole('listbox', { name: 'Trading pair', exact: true })).toBeVisible()
      const before = await geometry(page)
      await page.getByTestId('trade-pair-select-panel').evaluate((panel) => {
        panel.scrollTop = 24
      })
      await expect(async () => {
        const after = await geometry(page)
        expect(Math.abs(before.anchorTop - after.anchorTop - 24)).toBeLessThanOrEqual(TOLERANCE)
      }).toPass({ timeout: 5_000 })
      await assertTracksTrigger(page)
      await assertInsideChrome(page)
      const beforeMenuScroll = await geometry(page)
      await page.locator('.token-select-dropdown-list').evaluate((list) => {
        list.scrollTop = list.scrollHeight
      })
      await expect(async () => {
        expect(await page.locator('.token-select-dropdown-list').evaluate((list) => list.scrollTop)).toBeGreaterThan(0)
      }).toPass({ timeout: 5_000 })
      await expect(async () => {
        const after = await geometry(page)
        expect(Math.abs(after.menuTop - beforeMenuScroll.menuTop)).toBeLessThanOrEqual(TOLERANCE)
        expect(Math.abs(after.menuBottom - beforeMenuScroll.menuBottom)).toBeLessThanOrEqual(TOLERANCE)
      }).toPass({ timeout: 5_000 })
      await page.keyboard.press('Escape')
      await expect(page.locator('.token-select-dropdown')).toHaveCount(0)
    })

    test('scrolling the trigger behind the sticky header keeps the open menu in the usable band', async ({ page }) => {
      await page.locator('#trade-pair-select').click()
      await expect(page.getByRole('listbox', { name: 'Trading pair', exact: true })).toBeVisible()
      await page.evaluate(() => {
        const trigger = document.querySelector('#trade-pair-select')!.getBoundingClientRect()
        const header = document.querySelector('.app-top-sticky')!.getBoundingClientRect()
        window.scrollBy(0, trigger.bottom - header.bottom + 24)
      })
      await expect(async () => {
        const g = await geometry(page)
        expect(g.anchorBottom).toBeLessThan(g.headerBottom)
      }).toPass({ timeout: 5_000 })
      await assertInsideChrome(page)
      await page.getByRole('listbox', { name: 'Trading pair', exact: true }).getByRole('option').first().click()
      await expect(page.locator('.token-select-dropdown')).toHaveCount(0)
    })
  })
}
