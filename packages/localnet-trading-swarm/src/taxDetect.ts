/** Community-tax token detection for LocalTerra swarm (GitLab #621). */

export const DEFAULT_SELL_BPS = 500
export const TAX_BPS_DENOM = 10_000

/** Default on. `SWARM_TAX_WORKERS=0` / `false` is the exclude-only escape hatch. */
export function taxWorkersEnabled(env: NodeJS.ProcessEnv = process.env): boolean {
  const raw = (env.SWARM_TAX_WORKERS ?? '1').trim().toLowerCase()
  return raw !== '0' && raw !== 'false' && raw !== 'off' && raw !== 'no'
}

export function taxTokenFromEnv(env: Record<string, string | undefined>): string | undefined {
  const pinned = env.VITE_TOKEN_COMMUNITY_TAX_ADDRESS?.trim()
  return pinned && pinned.startsWith('terra1') ? pinned : undefined
}

export function normalizeTaxTokens(addrs: Iterable<string | undefined | null>): Set<string> {
  const out = new Set<string>()
  for (const a of addrs) {
    const t = a?.trim()
    if (t && t.startsWith('terra1')) out.add(t)
  }
  return out
}

export function isTaxToken(addr: string, taxTokens: Set<string>): boolean {
  return taxTokens.has(addr)
}
