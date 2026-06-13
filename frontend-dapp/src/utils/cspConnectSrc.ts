/** WalletConnect relay endpoints required by @goblinhunt/cosmes mobile wallets. */
const WALLETCONNECT_CONNECT_HOSTS = [
  'https://relay.walletconnect.com',
  'wss://relay.walletconnect.com',
  'https://relay.walletconnect.org',
  'wss://relay.walletconnect.org',
  'https://verify.walletconnect.com',
  'https://verify.walletconnect.org',
]

function originFromUrl(raw: string | undefined): string | null {
  if (!raw?.trim()) return null
  try {
    const url = new URL(raw.trim())
    return url.origin
  } catch {
    return null
  }
}

function rpcConnectOrigins(raw: string | undefined): string[] {
  if (!raw?.trim()) return []
  try {
    const url = new URL(raw.trim())
    const origins = [url.origin]
    if (url.protocol === 'https:') {
      origins.push(`wss://${url.host}`)
    } else if (url.protocol === 'http:') {
      origins.push(`ws://${url.host}`)
    }
    return origins
  } catch {
    return []
  }
}

/**
 * Production CSP `connect-src` allowlist from deploy env (GitLab #378 / M-07).
 * Dev `vite` keeps broad `https:` / `wss:` in index.html; this narrows production builds only.
 */
export function buildProductionConnectSrc(env: Record<string, string>): string {
  const hosts = new Set<string>(["'self'", ...WALLETCONNECT_CONNECT_HOSTS])

  for (const origin of [
    originFromUrl(env.VITE_TERRA_LCD_URL),
    ...rpcConnectOrigins(env.VITE_TERRA_RPC_URL),
    originFromUrl(env.VITE_INDEXER_URL),
  ]) {
    if (origin) hosts.add(origin)
  }

  // Mainnet/testnet fallbacks when operators omit explicit URLs at build time.
  if (!env.VITE_TERRA_LCD_URL?.trim()) {
    hosts.add('https://terra-classic-lcd.publicnode.com')
  }
  if (!env.VITE_TERRA_RPC_URL?.trim()) {
    hosts.add('https://terra-classic-rpc.publicnode.com:443')
    hosts.add('wss://terra-classic-rpc.publicnode.com:443')
  }

  return [...hosts].join(' ')
}
