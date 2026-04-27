import { existsSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

/** Repo root: contains `docker-compose.yml` and `frontend-dapp/`. */
export function getRepoRoot(): string {
  const envRoot = process.env.SWARM_REPO_ROOT?.trim()
  if (envRoot && existsSync(join(envRoot, 'docker-compose.yml')) && existsSync(join(envRoot, 'frontend-dapp'))) {
    return envRoot
  }
  const here = dirname(fileURLToPath(import.meta.url))
  // src/ -> package -> packages -> repo
  let dir = join(here, '..', '..', '..')
  for (let i = 0; i < 6; i++) {
    if (existsSync(join(dir, 'docker-compose.yml')) && existsSync(join(dir, 'frontend-dapp'))) {
      return dir
    }
    const up = dirname(dir)
    if (up === dir) break
    dir = up
  }
  throw new Error(
    'Could not find repository root (expected docker-compose.yml and frontend-dapp/). Run from the cl8y-dex-terraclassic clone.'
  )
}
