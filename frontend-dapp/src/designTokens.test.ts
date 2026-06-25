import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..', '..')

const BLUE_HEX_RE = /#(?:3b82f6|2563eb|60a5fa|38bdf8|0f172a|1e293b|334155)/i

describe('design token alignment (GitLab #416)', () => {
  it('tailwind.config.js maps colors to CSS variables, not legacy blue hex', () => {
    const config = readFileSync(join(repoRoot, 'frontend-dapp/tailwind.config.js'), 'utf8')
    expect(config).not.toMatch(BLUE_HEX_RE)
    expect(config).toContain("0: 'var(--bg-0)'")
    expect(config).toContain("DEFAULT: 'var(--ink)'")
    expect(config).not.toContain('primary:')
    expect(config).not.toContain('dex:')
  })

  it('trade-bootstrap.css uses theme tokens, not hard-coded blues', () => {
    const css = readFileSync(join(repoRoot, 'frontend-dapp/public/bootstrap/trade-bootstrap.css'), 'utf8')
    expect(css).not.toMatch(BLUE_HEX_RE)
    expect(css).toContain('background: var(--bg-0)')
    expect(css).toContain('border: 1px solid var(--line)')
    expect(css).toContain('--bg-0: #0e0908')
    expect(css).toContain(":root[data-theme='light']")
  })

  it('trade-bootstrap dark bg-0 matches theme-dark.css', () => {
    const bootstrap = readFileSync(join(repoRoot, 'frontend-dapp/public/bootstrap/trade-bootstrap.css'), 'utf8')
    const themeDark = readFileSync(join(repoRoot, 'frontend-dapp/src/theme-dark.css'), 'utf8')
    const bg0 = /--bg-0:\s*([^;]+);/.exec(themeDark)?.[1]?.trim()
    expect(bg0).toBeTruthy()
    expect(bootstrap).toContain(`--bg-0: ${bg0}`)
  })

  it('trade-bootstrap light bg-0 matches theme-light.css', () => {
    const bootstrap = readFileSync(join(repoRoot, 'frontend-dapp/public/bootstrap/trade-bootstrap.css'), 'utf8')
    const themeLight = readFileSync(join(repoRoot, 'frontend-dapp/src/theme-light.css'), 'utf8')
    const bg0 = /--bg-0:\s*([^;]+);/.exec(themeLight)?.[1]?.trim()
    expect(bg0).toBeTruthy()
    expect(bootstrap).toContain(`--bg-0: ${bg0}`)
  })
})
