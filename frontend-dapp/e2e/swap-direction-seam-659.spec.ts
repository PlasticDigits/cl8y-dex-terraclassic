import { mkdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test, expect, type Locator, type Page } from '@playwright/test'

/**
 * Swap direction seam paint (GitLab #659).
 * UI-only: PLAYWRIGHT_SKIP_CHAIN=1. Workers stay at 5 in playwright.config.ts.
 *
 * LcdQueryGate hides Swap until factory pairs load. When `/` has no factory pin
 * (worktree without `.env.local`), inject the same seam classes so we still
 * assert production CSS. Prefer the live Swap control when it mounts.
 */
const SCREENSHOT_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'docs', 'qa', 'issue-659')

const FIXTURE_HTML = `
<div class="swap-io-stack relative" id="swap-direction-seam-fixture" data-testid="swap-direction-seam-fixture">
  <div class="card-glass swap-io-card-pay" style="padding:1.25rem">You Pay</div>
  <div class="swap-direction-seam relative z-20 flex justify-center pointer-events-none" style="margin-top:-1.25rem;margin-bottom:-1.25rem">
    <button type="button" class="pointer-events-auto swap-direction-btn w-10 h-10 rounded-2xl flex items-center justify-center" aria-label="Swap pay and receive tokens">
      <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
        <path d="M8 1v14M8 1L4 5M8 1l4 4M8 15l-4-4M8 15l4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </button>
  </div>
  <div class="card-glass swap-io-card-receive" style="padding:1.25rem">You Receive</div>
</div>
`

function isOpaqueCssColor(color: string): boolean {
  const rgb = /^rgb\(\s*\d+\s*,\s*\d+\s*,\s*\d+\s*\)$/.test(color)
  if (rgb) return true
  const rgba = /^rgba\(\s*\d+\s*,\s*\d+\s*,\s*\d+\s*,\s*([\d.]+)\s*\)$/.exec(color)
  return rgba != null && Number(rgba[1]) >= 0.99
}

async function setTheme(page: Page, theme: 'dark' | 'light') {
  await page.addInitScript((t) => {
    localStorage.setItem('cl8y-dex-theme', t)
  }, theme)
  await page.goto('/')
  await page.waitForLoadState('domcontentloaded')
  await page.evaluate((t) => {
    document.documentElement.setAttribute('data-theme', t)
    localStorage.setItem('cl8y-dex-theme', t)
  }, theme)
}

async function ensureSeamMounted(page: Page): Promise<Locator> {
  const live = page.getByRole('button', { name: 'Swap pay and receive tokens' })
  try {
    await live.waitFor({ state: 'visible', timeout: 8_000 })
    return live
  } catch {
    await page.evaluate((html) => {
      if (document.getElementById('swap-direction-seam-fixture')) return
      const main = document.querySelector('main')
      if (main instanceof HTMLElement) main.style.visibility = 'hidden'
      const wrap = document.createElement('div')
      wrap.style.cssText = 'position:fixed;inset:80px 16px auto;z-index:5;max-width:500px;margin:0 auto;left:0;right:0'
      wrap.innerHTML = html
      document.body.appendChild(wrap)
    }, FIXTURE_HTML)
    const injected = page.locator('#swap-direction-seam-fixture').getByRole('button', {
      name: 'Swap pay and receive tokens',
    })
    await expect(injected).toBeVisible({ timeout: 5_000 })
    return injected
  }
}

test.describe('Swap direction seam (GitLab #659)', () => {
  test.beforeAll(() => {
    mkdirSync(SCREENSHOT_DIR, { recursive: true })
  })

  for (const theme of ['dark', 'light'] as const) {
    for (const viewport of [
      { width: 375, height: 812, label: '375' },
      { width: 1280, height: 800, label: '1280' },
    ] as const) {
      test(`${theme} ${viewport.label}px: opaque plate, static occluder, hairline remains`, async ({ page }) => {
        await page.setViewportSize({ width: viewport.width, height: viewport.height })
        await setTheme(page, theme)
        const btn = await ensureSeamMounted(page)

        const paint = await btn.evaluate((el) => {
          const s = getComputedStyle(el)
          const seam = el.parentElement
          const occluder = seam ? getComputedStyle(seam, '::before') : null
          const pay = el.closest('.swap-io-stack')?.querySelector('.swap-io-card-pay')
          const payBorder = pay ? getComputedStyle(pay).borderBottom : ''
          return {
            backgroundColor: s.backgroundColor,
            transform: s.transform,
            pointerEvents: s.pointerEvents,
            wrapperPointerEvents: seam ? getComputedStyle(seam).pointerEvents : '',
            wrapperZ: seam ? getComputedStyle(seam).zIndex : '',
            occluderContent: occluder?.content ?? '',
            occluderBg: occluder?.backgroundColor ?? '',
            payBorder,
            ariaHiddenSvg: el.querySelector('svg')?.getAttribute('aria-hidden'),
          }
        })

        expect(isOpaqueCssColor(paint.backgroundColor), paint.backgroundColor).toBe(true)
        expect(paint.transform === 'none' || paint.transform === 'matrix(1, 0, 0, 1, 0, 0)').toBe(true)
        expect(paint.pointerEvents).toBe('auto')
        expect(paint.wrapperPointerEvents).toBe('none')
        expect(Number(paint.wrapperZ)).toBeLessThanOrEqual(20)
        expect(paint.occluderContent.replace(/['"]/g, '')).not.toBe('none')
        expect(isOpaqueCssColor(paint.occluderBg), paint.occluderBg).toBe(true)
        expect(paint.payBorder).toMatch(/1px/)
        expect(paint.ariaHiddenSvg).toBeTruthy()

        await btn.hover()
        const hovered = await btn.evaluate((el) => {
          const s = getComputedStyle(el)
          return { backgroundColor: s.backgroundColor, transform: s.transform }
        })
        expect(isOpaqueCssColor(hovered.backgroundColor), hovered.backgroundColor).toBe(true)
        expect(hovered.transform === 'none' || hovered.transform === 'matrix(1, 0, 0, 1, 0, 0)').toBe(true)

        const stack = page.locator('.swap-io-stack').first()
        await stack.screenshot({
          path: join(SCREENSHOT_DIR, `seam-${theme}-${viewport.label}.png`),
        })
      })
    }
  }

  test('keyboard :focus-visible uses --focus-ring; click does not leave a ring', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 })
    await setTheme(page, 'dark')
    const btn = await ensureSeamMounted(page)

    await page.mouse.click(8, 8)
    await btn.click()
    expect(await btn.evaluate((el) => el.matches(':focus-visible'))).toBe(false)

    await page.locator('body').click({ position: { x: 2, y: 2 } })
    for (let i = 0; i < 24; i++) {
      const focused = await page.evaluate(
        () => document.activeElement?.getAttribute('aria-label') === 'Swap pay and receive tokens'
      )
      if (focused) break
      await page.keyboard.press('Tab')
    }
    await expect(btn).toBeFocused()
    const kb = await btn.evaluate((el) => ({
      shadow: getComputedStyle(el).boxShadow,
      focusVisible: el.matches(':focus-visible'),
    }))
    expect(kb.focusVisible).toBe(true)
    expect(kb.shadow).not.toBe('none')
  })
})
