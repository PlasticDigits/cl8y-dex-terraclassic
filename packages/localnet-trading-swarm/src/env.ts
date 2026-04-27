import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

export interface SwarmEnv {
  VITE_NETWORK?: string
  VITE_FACTORY_ADDRESS: string
  VITE_ROUTER_ADDRESS: string
  VITE_TERRA_LCD_URL: string
  VITE_TERRA_RPC_URL: string
  VITE_TREASURY_ADDRESS?: string
  VITE_WRAP_MAPPER_ADDRESS?: string
  VITE_LUNC_C_TOKEN_ADDRESS?: string
  VITE_USTC_C_TOKEN_ADDRESS?: string
  VITE_CL8Y_TOKEN_ADDRESS?: string
  VITE_GAS_PRICE_ULUNA?: string
  [key: string]: string | undefined
}

/** Parse `frontend-dapp/.env.local` for `VITE_*` exports; process.env overrides. */
export function loadViteEnv(repoRoot: string, envPath?: string): SwarmEnv {
  const path = envPath ? resolve(envPath) : resolve(repoRoot, 'frontend-dapp', '.env.local')
  const merged: Record<string, string> = {}

  let content: string
  try {
    content = readFileSync(path, 'utf8')
  } catch {
    throw new Error(
      `Missing ${path}. Run scripts/deploy-dex-local.sh first (writes frontend-dapp/.env.local), or set VITE_* overrides in the environment.`
    )
  }

  for (const line of content.split('\n')) {
    const t = line.trim()
    if (!t || t.startsWith('#')) continue
    const m = /^VITE_[A-Z0-9_]+=/.exec(t)
    if (!m) continue
    const eq = t.indexOf('=')
    const key = t.slice(0, eq)
    let val = t.slice(eq + 1).trim()
    if (
      (val.startsWith('"') && val.endsWith('"')) ||
      (val.startsWith("'") && val.endsWith("'"))
    ) {
      val = val.slice(1, -1)
    }
    if (key.startsWith('VITE_')) merged[key] = val
  }

  for (const [k, v] of Object.entries(process.env)) {
    if (k.startsWith('VITE_') && typeof v === 'string' && v.length > 0) {
      merged[k] = v
    }
  }

  return merged as SwarmEnv
}
