import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..', '..')

const OPAQUE_COLOR =
  /^(?:rgb\(\s*\d+\s*,\s*\d+\s*,\s*\d+\s*\)|#[0-9a-fA-F]{6}|rgba\(\s*\d+\s*,\s*\d+\s*,\s*\d+\s*,\s*1(?:\.0+)?\s*\))$/

function tokenValue(css: string, name: string): string | undefined {
  return new RegExp(`--${name}:\\s*([^;]+);`).exec(css)?.[1]?.trim()
}

describe('Swap direction seam plate (GitLab #659)', () => {
  const indexCss = readFileSync(join(repoRoot, 'frontend-dapp/src/index.css'), 'utf8')
  const swapPage = readFileSync(join(repoRoot, 'frontend-dapp/src/pages/SwapPage.tsx'), 'utf8')
  const themeDark = readFileSync(join(repoRoot, 'frontend-dapp/src/theme-dark.css'), 'utf8')
  const themeLight = readFileSync(join(repoRoot, 'frontend-dapp/src/theme-light.css'), 'utf8')

  it('defines opaque --swap-direction-surface in both themes (S659-1 / S659-2)', () => {
    for (const [label, css] of [
      ['dark', themeDark],
      ['light', themeLight],
    ] as const) {
      for (const token of ['swap-direction-surface', 'swap-direction-surface-hover'] as const) {
        const value = tokenValue(css, token)
        expect(value, `${label} --${token}`).toBeDefined()
        expect(value!.replace(/\s+/g, ' '), `${label} --${token} opaque`).toMatch(OPAQUE_COLOR)
      }
    }
  })

  it('uses the opaque plate token, not --control-surface glass wash', () => {
    const btn = /\.swap-direction-btn\s*\{[^}]+\}/.exec(indexCss)?.[0] ?? ''
    expect(btn).toContain('var(--swap-direction-surface)')
    expect(btn).not.toContain('--control-surface')
    const hover = /\.swap-direction-btn:hover\s*\{[^}]+\}/.exec(indexCss)?.[0] ?? ''
    expect(hover).toContain('var(--swap-direction-surface-hover)')
    expect(hover).not.toContain('--control-surface')
  })

  it('keeps a static seam occluder that does not translate (S659-3)', () => {
    expect(indexCss).toContain('.swap-direction-seam::before')
    const occluder = /\.swap-direction-seam::before\s*\{[^}]+\}/.exec(indexCss)?.[0] ?? ''
    expect(occluder).toContain('var(--swap-direction-surface)')
    expect(occluder).toContain('pointer-events: none')
    expect(indexCss).not.toMatch(/\.swap-direction-btn:hover[\s\S]{0,120}translate/)
    expect(swapPage).not.toContain('hover:-translate-y')
  })

  it('adds :focus-visible ring only (S659-4)', () => {
    expect(indexCss).toContain('.swap-direction-btn:focus-visible')
    expect(indexCss).not.toMatch(/\.swap-direction-btn:focus[^-]/)
    const ring = /\.swap-direction-btn:focus-visible\s*\{[^}]+\}/.exec(indexCss)?.[0] ?? ''
    expect(ring).toContain('var(--focus-ring)')
    expect(ring).toContain('outline: none')
  })

  it('keeps flip JS, aria-label, and static SVG (S659-5)', () => {
    expect(swapPage).toContain('aria-label="Swap pay and receive tokens"')
    expect(swapPage).toContain('setFromToken(toToken)')
    expect(swapPage).toContain('setToToken(tmp)')
    expect(swapPage).not.toContain('dangerouslySetInnerHTML')
    expect(swapPage).toMatch(/<svg[^>]*aria-hidden/)
  })

  it('keeps the Pay/Receive hairline as IA (S659-6)', () => {
    expect(indexCss).toMatch(
      /\.swap-io-card-pay\.card-glass \{\s*border-radius: 24px 24px 0 0;\s*border-bottom: 1px solid var\(--chrome-border\);/
    )
    expect(swapPage).toContain('card-glass swap-io-card-pay')
    expect(swapPage).toContain('card-glass swap-io-card-receive')
  })

  it('does not add chrome or raise hit-stealing z-index (S659-7 / S659-8)', () => {
    expect(swapPage).not.toMatch(/swap-direction-btn[^"]*-neo/)
    expect(swapPage).not.toMatch(/swap-direction-seam[^"]*card-glass/)
    expect(swapPage).toContain('pointer-events-none')
    expect(swapPage).toContain('pointer-events-auto')
    expect(swapPage).toContain('z-20')
    expect(swapPage).not.toMatch(/swap-direction-seam[^"]*z-(?:[3-9]\d|50|\[)/)
    expect(indexCss).toMatch(/\.swap-direction-seam\s*\{[^}]*z-index:\s*20/s)
  })
})
