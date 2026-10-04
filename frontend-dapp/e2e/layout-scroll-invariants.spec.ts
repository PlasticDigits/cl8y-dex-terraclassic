import { expect, test } from '@playwright/test'
import { installScrollInvariantMarket, SCROLL_PAIR_ADDRESS } from './helpers/scroll-invariants-fixture'

test.setTimeout(120_000)

for (const viewport of [
  { width: 1280, height: 720 },
  { width: 375, height: 667 },
]) {
  for (const theme of ['dark', 'light']) {
    test(`sticky header stays opaque and reachable after scroll: ${viewport.width}px ${theme} (#1316)`, async ({
      page,
    }) => {
      await page.setViewportSize(viewport)
      await page.addInitScript((value) => localStorage.setItem('cl8y-dex-theme', value), theme)
      await installScrollInvariantMarket(page)
      await page.goto(`/trade/${SCROLL_PAIR_ADDRESS}`)
      const heading = page.getByRole('heading', { name: /^trade$/i })
      await expect(heading).toBeVisible({ timeout: 90_000 })
      const header = page.locator('.app-top-sticky')
      const headingBox = await heading.boundingBox()
      const headerBox = await header.boundingBox()
      // The documented ~16px clearance allows the compact shell's 14px spacing.
      expect(headingBox!.y - (headerBox!.y + headerBox!.height)).toBeGreaterThanOrEqual(16 - 2)

      await page.evaluate(() => window.scrollTo(0, 160))
      await expect.poll(() => page.evaluate(() => window.scrollY)).toBeGreaterThan(0)
      await expect.poll(async () => (await header.boundingBox())!.y).toBeCloseTo(0, 0)
      const opacity = await header.evaluate((el) => {
        const style = getComputedStyle(el)
        const canvas = document.createElement('canvas')
        canvas.width = canvas.height = 1
        const ctx = canvas.getContext('2d')!
        ctx.fillStyle = style.backgroundColor
        ctx.fillRect(0, 0, 1, 1)
        return { alpha: ctx.getImageData(0, 0, 1, 1).data[3], opacity: style.opacity }
      })
      expect(opacity).toEqual({ alpha: 255, opacity: '1' })
      // Hit testing verifies that page content passing below the header cannot steal a tap.
      const brand = page.locator('.app-brand')
      expect(
        await brand.evaluate((el) => {
          const r = el.getBoundingClientRect()
          return el.contains(document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2))
        })
      ).toBe(true)
      await brand.click()
      await expect(page).toHaveURL(/\/$/)
    })
  }
}

test('Limit CTA stays docked and misses visible controls through ticket scroll (#1316)', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 })
  await installScrollInvariantMarket(page)
  await page.goto(`/trade/${SCROLL_PAIR_ADDRESS}`)
  await expect(page.getByTestId('trade-desktop-workspace')).toBeVisible({ timeout: 90_000 })
  await page.getByTestId('trade-order-tab-limit').click()
  await page.locator('summary', { hasText: 'Advanced' }).first().click()
  const scroll = page.getByTestId('trade-order-ticket-scroll')
  const footer = page.getByTestId('trade-ticket-submit-footer')
  // Disconnected users see Connect Wallet in the same money-CTA slot.
  const cta = footer.getByRole('button').first()
  await expect(cta).toBeVisible()
  const maxScroll = await scroll.evaluate((el) => el.scrollHeight - el.clientHeight)
  expect(maxScroll, 'must actually exercise an overflowing ticket').toBeGreaterThan(0)

  for (const fraction of [0, 0.5, 1]) {
    await scroll.evaluate((el, top) => {
      el.scrollTop = top
    }, maxScroll * fraction)
    await expect.poll(() => scroll.evaluate((el) => el.scrollTop)).toBe(Math.round(maxScroll * fraction))
    const geometry = await scroll.evaluate((el) => {
      const card = el.closest('[data-testid="trade-order-ticket-card"]')!
      const footer = card.querySelector('[data-testid="trade-ticket-submit-footer"]')!
      const cta = footer.querySelector('button')!
      const sr = el.getBoundingClientRect()
      const cr = cta.getBoundingClientRect()
      const controls = [...el.querySelectorAll('input, button, summary, select')]
      const visible = controls.flatMap((control) => {
        const r = control.getBoundingClientRect()
        const left = Math.max(r.left, sr.left)
        const right = Math.min(r.right, sr.right)
        const top = Math.max(r.top, sr.top)
        const bottom = Math.min(r.bottom, sr.bottom)
        if (right <= left || bottom <= top) return []
        return [{ overlap: left < cr.right && right > cr.left && top < cr.bottom && bottom > cr.top }]
      })
      return {
        dockGap: Math.abs(cr.bottom - card.getBoundingClientRect().bottom),
        visibleCount: visible.length,
        overlaps: visible.filter((item) => item.overlap).length,
        footerPosition: getComputedStyle(footer).position,
        ctaPosition: getComputedStyle(cta).position,
        sibling: footer.parentElement === el.parentElement,
      }
    })
    expect(geometry.dockGap).toBeLessThanOrEqual(8)
    expect(geometry.visibleCount).toBeGreaterThan(0)
    expect(geometry.overlaps).toBe(0)
    expect(geometry.sibling).toBe(true)
    expect(['sticky', 'fixed']).not.toContain(geometry.footerPosition)
    expect(['sticky', 'fixed']).not.toContain(geometry.ctaPosition)
  }
})
