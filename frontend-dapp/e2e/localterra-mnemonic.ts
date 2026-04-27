import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import path from 'node:path'

/**
 * Single source of truth for the LocalTerra test account: `TEST_MNEMONIC` in docker/init-chain.sh.
 * E2E injects this at the Playwright `webServer` so it never needs to be duplicated in frontend source
 * (GitLab #118).
 */
export function getLocalTerraTestMnemonic(): string {
  const fromProcess = process.env.VITE_DEV_MNEMONIC?.trim()
  if (fromProcess) {
    return fromProcess
  }
  const here = path.dirname(fileURLToPath(import.meta.url))
  const repoRoot = path.resolve(here, '..', '..')
  const init = path.join(repoRoot, 'docker', 'init-chain.sh')
  const text = readFileSync(init, 'utf8')
  const m = text.match(/^TEST_MNEMONIC="([^"]+)"\s*$/m)
  if (!m) {
    throw new Error(
      `Could not find TEST_MNEMONIC in ${init} — set VITE_DEV_MNEMONIC in the environment or keep docker/init-chain.sh in sync.`
    )
  }
  return m[1]!
}
