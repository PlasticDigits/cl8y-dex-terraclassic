import type { Locator, Page } from '@playwright/test'
import { expect } from '@playwright/test'

/** Header wallet control when disconnected (matches WalletButton copy). */
export function headerConnectButton(page: Page) {
  const header = page.locator('header')
  return header.getByRole('button', { name: 'Connect Wallet' }).or(header.getByRole('button', { name: 'Connect' }))
}

/** Header wallet control when connected (shortened address in button label). */
export function headerConnectedWalletButton(page: Page) {
  return page
    .locator('header')
    .getByRole('button')
    .filter({ hasText: /terra1/ })
}

export const WALLET_MENU_ACTION_NAMES = [
  'View on explorer',
  'Switch wallet',
  'My Portfolio',
  'Trader profile',
  'Disconnect',
] as const

/** Connected chip dropdown under the sticky header. */
export function headerWalletMenu(page: Page) {
  return page.locator('header.app-header-shell .wallet-menu')
}

/**
 * Icon left of visible label, sharing a horizontal band (GitLab #671).
 * Uses the first non-empty text node so sr-only live regions are ignored.
 */
export async function expectWalletMenuItemHorizontalRow(item: Locator) {
  const metrics = await item.evaluate((el) => {
    const svg = el.querySelector('svg')
    if (!svg) return null
    const icon = svg.getBoundingClientRect()
    const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT)
    let label: DOMRect | null = null
    let node: Node | null
    while ((node = walker.nextNode())) {
      const parent = node.parentElement
      if (parent?.classList.contains('sr-only')) continue
      const text = node.textContent?.trim()
      if (!text) continue
      const range = document.createRange()
      range.selectNodeContents(node)
      label = range.getBoundingClientRect()
      break
    }
    return {
      icon: { x: icon.x, y: icon.y, w: icon.width, h: icon.height, cy: icon.y + icon.height / 2 },
      label: label ? { x: label.x, y: label.y, w: label.width, h: label.height, cy: label.y + label.height / 2 } : null,
      rowH: el.getBoundingClientRect().height,
    }
  })
  expect(metrics, 'menuitem icon + label metrics').toBeTruthy()
  expect(metrics!.label, 'visible label box').toBeTruthy()
  expect(metrics!.icon.x).toBeLessThan(metrics!.label!.x)
  expect(Math.abs(metrics!.icon.cy - metrics!.label!.cy)).toBeLessThanOrEqual(4)
  // Padding 11+11 plus 16px icon ≈ 38–46px; stacked icon-over-label is ~60px+.
  expect(metrics!.rowH).toBeLessThan(metrics!.icon.h + 40)
}
