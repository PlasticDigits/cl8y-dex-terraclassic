import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..', '..')

const LEGACY_TAILWIND_BLUE_RE = /#(?:3b82f6|2563eb|60a5fa|38bdf8|0f172a|1e293b|334155)/i
const WARM_BROWN_BG_RE = /#(?:0e0908|f4e0cb|1d110f)/i
const BROWN_WARNING_FILL_RE = /rgba\(\s*74,\s*(?:40|55),\s*12\b/
const GOLD_PAGE_WASH_RE = /rgba\(\s*232,\s*184,\s*74\b/

describe('design token alignment (GitLab #488 blue+gold)', () => {
  it('tailwind.config.js maps colors to CSS variables, not legacy hex palettes', () => {
    const config = readFileSync(join(repoRoot, 'frontend-dapp/tailwind.config.js'), 'utf8')
    expect(config).not.toMatch(LEGACY_TAILWIND_BLUE_RE)
    expect(config).toContain("0: 'var(--bg-0)'")
    expect(config).toContain("DEFAULT: 'var(--ink)'")
    expect(config).toContain("DEFAULT: 'var(--blue)'")
    expect(config).toContain("DEFAULT: 'var(--gold)'")
    expect(config).not.toContain('primary:')
    expect(config).not.toContain('dex:')
  })

  it('trade-bootstrap.css uses cool theme tokens, not warm brown', () => {
    const css = readFileSync(join(repoRoot, 'frontend-dapp/public/bootstrap/trade-bootstrap.css'), 'utf8')
    expect(css).not.toMatch(WARM_BROWN_BG_RE)
    expect(css).toContain('background: var(--bg-0)')
    expect(css).toContain('border: 1px solid var(--line)')
    expect(css).toContain('--bg-0: #0d111c')
    expect(css).toContain(":root[data-theme='light']")
    expect(css).toContain('--bg-0: #f4f6fb')
  })

  it('trade-bootstrap dark bg-0 matches theme-dark.css', () => {
    const bootstrap = readFileSync(join(repoRoot, 'frontend-dapp/public/bootstrap/trade-bootstrap.css'), 'utf8')
    const themeDark = readFileSync(join(repoRoot, 'frontend-dapp/src/theme-dark.css'), 'utf8')
    const bg0 = /--bg-0:\s*([^;]+);/.exec(themeDark)?.[1]?.trim()
    expect(bg0).toBe('#0d111c')
    expect(bootstrap).toContain(`--bg-0: ${bg0}`)
  })

  it('trade-bootstrap light bg-0 matches theme-light.css', () => {
    const bootstrap = readFileSync(join(repoRoot, 'frontend-dapp/public/bootstrap/trade-bootstrap.css'), 'utf8')
    const themeLight = readFileSync(join(repoRoot, 'frontend-dapp/src/theme-light.css'), 'utf8')
    const bg0 = /--bg-0:\s*([^;]+);/.exec(themeLight)?.[1]?.trim()
    expect(bg0).toBe('#f4f6fb')
    expect(bootstrap).toContain(`--bg-0: ${bg0}`)
  })

  it('theme files define blue primary and gold brand accents', () => {
    for (const file of ['theme-dark.css', 'theme-light.css'] as const) {
      const css = readFileSync(join(repoRoot, 'frontend-dapp/src', file), 'utf8')
      expect(css).toContain('--blue: #448aff')
      expect(css).toContain('--gold: #e8b84a')
      expect(css).toContain('--mint: var(--blue)')
      expect(css).not.toMatch(BROWN_WARNING_FILL_RE)
    }
  })

  it('index.css avoids brown warning fills and gold page washes (#488)', () => {
    const css = readFileSync(join(repoRoot, 'frontend-dapp/src/index.css'), 'utf8')
    expect(css).not.toMatch(BROWN_WARNING_FILL_RE)
    const bodyBlock = /body\s*\{[^}]*background:[^}]*\}/s.exec(css)?.[0] ?? ''
    expect(bodyBlock).not.toMatch(GOLD_PAGE_WASH_RE)
    expect(css).toMatch(/\.app-nav-link-active[\s\S]*background:\s*var\(--accent-surface\)/)
    expect(css).not.toMatch(/\.app-nav-link-active[\s\S]*background:\s*var\(--gold-surface\)/)
    expect(css).not.toMatch(/rgba\(\s*249,\s*115,\s*22\b/)
    expect(css).not.toMatch(/\.app-modal-backdrop[\s\S]{0,200}rgba\(\s*7,\s*4,\s*3\b/)
  })

  it('Buy/Sell side controls use semantic fills, not alert-error or tab-glass-active (#563)', () => {
    const css = readFileSync(join(repoRoot, 'frontend-dapp/src/index.css'), 'utf8')
    expect(css).toContain('.side-buy-selected')
    expect(css).toContain('.side-sell-idle')
    expect(css).toContain('.side-control:focus-visible')
    expect(css).toContain('.trade-ticket-heading')
    expect(css).toContain('overflow-wrap: anywhere')
    const sellIdle = /\.side-sell-idle\s*\{[^}]+\}/.exec(css)?.[0] ?? ''
    expect(sellIdle).not.toContain('alert-error')
    expect(sellIdle).toContain('var(--side-sell-fg)')
    const heading = /\.trade-ticket-heading\s*\{[^}]+\}/.exec(css)?.[0] ?? ''
    expect(heading).not.toContain('ellipsis')
    expect(heading).not.toContain('truncate')
  })

  it('chart interval active chip is stronger than 14% tab-glass wash (#705)', () => {
    const css = readFileSync(join(repoRoot, 'frontend-dapp/src/index.css'), 'utf8')
    expect(css).toContain('.price-chart-interval.tab-glass-active')
    expect(css).toContain(":root[data-theme='light'] .price-chart-interval.tab-glass-active")
    expect(css).toContain('.price-chart-interval.tab-glass.tab-glass-active:focus-visible')
    const intervalActive = /\.price-chart-interval\.tab-glass-active\s*\{[^}]+\}/.exec(css)?.[0] ?? ''
    expect(intervalActive).toContain('var(--blue)')
    expect(intervalActive).not.toContain('0.14')
    expect(intervalActive).not.toContain('--gold')
    expect(intervalActive).not.toContain('btn-primary')
    expect(css).not.toMatch(/price-chart-interval[^{]*-neo/)
  })
})
