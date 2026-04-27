/** @vitest-environment node */
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { loadConfigFromFile } from 'vite'
import { describe, expect, it } from 'vitest'

const viteConfigPath = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', 'vite.config.ts')

describe('vite.config production source maps', () => {
  it('disables build.sourcemap for production mode (GitLab #117)', async () => {
    const loaded = await loadConfigFromFile({ command: 'build', mode: 'production' }, viteConfigPath)
    expect(loaded?.config.build?.sourcemap).toBe(false)
  })

  it('enables build.sourcemap for non-production build modes', async () => {
    const loaded = await loadConfigFromFile({ command: 'build', mode: 'staging' }, viteConfigPath)
    expect(loaded?.config.build?.sourcemap).toBe(true)
  })
})
