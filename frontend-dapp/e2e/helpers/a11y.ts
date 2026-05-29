import AxeBuilder from '@axe-core/playwright'
import type { Page } from '@playwright/test'
import { expect } from '@playwright/test'

/** WCAG 2.1 Level A + AA tags for retail-critical route scans (GitLab #214). */
export const AXE_WCAG_TAGS = ['wcag2a', 'wcag2aa'] as const

export type A11yScanOptions = {
  /** axe `include` selectors (scoped scan). */
  include?: string[]
  /** axe `exclude` selectors — canvas/decorative only; not interactive nodes. */
  exclude?: string[]
}

/**
 * Fail when axe reports critical or serious violations.
 * Document intentional rule disables in the spec comment + docs/frontend.md.
 */
export async function assertNoCriticalA11yViolations(page: Page, options: A11yScanOptions = {}) {
  let builder = new AxeBuilder({ page }).withTags([...AXE_WCAG_TAGS])

  if (options.include?.length) {
    builder = builder.include(options.include)
  }
  if (options.exclude?.length) {
    builder = builder.exclude(options.exclude)
  }

  const results = await builder.analyze()

  const blocking = results.violations.filter((v) => v.impact === 'critical' || v.impact === 'serious')
  expect(
    blocking,
    blocking.length
      ? `axe violations:\n${blocking.map((v) => `${v.id} (${v.impact}): ${v.help}\n${v.nodes.map((n) => n.html).join('\n')}`).join('\n\n')}`
      : undefined
  ).toEqual([])
}
