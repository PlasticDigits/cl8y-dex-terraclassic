import { defineConfig, loadEnv } from 'vite'
import type { Plugin } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'
import { execSync } from 'child_process'
import { networkInterfaces } from 'os'

let gitSha = 'dev'
try {
  gitSha = execSync('git rev-parse --short HEAD').toString().trim()
} catch {
  /* not in git repo yet */
}

function isPrivateIP(addr: string): boolean {
  return /^(10\.|172\.(1[6-9]|2\d|3[01])\.|192\.168\.|fd[0-9a-f]{2}:|fe80:)/.test(addr)
}

const WALLETCONNECT_CONNECT_ORIGINS = [
  'https://relay.walletconnect.com',
  'https://relay.walletconnect.org',
  'wss://relay.walletconnect.com',
  'wss://relay.walletconnect.org',
  'https://registry.walletconnect.com',
  'https://explorer-api.walletconnect.com',
]

const TRUSTED_LOGO_IMG_ORIGINS = [
  'https://gitlab.com',
  'https://raw.githubusercontent.com',
  'https://assets.coingecko.com',
  'https://ipfs.io',
  'https://cloudflare-ipfs.com',
]

function originFromUrl(raw: string | undefined): string | undefined {
  const trimmed = raw?.trim()
  if (!trimmed) return undefined
  try {
    const u = new URL(trimmed)
    return u.origin
  } catch {
    return undefined
  }
}

function buildProductionCsp(env: Record<string, string>): string {
  const connectOrigins = new Set<string>(["'self'", ...WALLETCONNECT_CONNECT_ORIGINS])
  for (const key of ['VITE_TERRA_LCD_URL', 'VITE_TERRA_RPC_URL', 'VITE_INDEXER_URL'] as const) {
    const origin = originFromUrl(env[key])
    if (origin) connectOrigins.add(origin)
  }

  const imgOrigins = new Set<string>(["'self'", 'data:', ...TRUSTED_LOGO_IMG_ORIGINS])

  return [
    "default-src 'self'",
    "script-src 'self'",
    "style-src 'self' 'unsafe-inline'",
    `connect-src ${[...connectOrigins].join(' ')}`,
    `img-src ${[...imgOrigins].join(' ')}`,
    "font-src 'self'",
    "object-src 'none'",
    "base-uri 'self'",
    "form-action 'self'",
    "frame-ancestors 'none'",
  ].join('; ')
}

function buildDevCsp(): string {
  return [
    "default-src 'self'",
    "script-src 'self' 'unsafe-inline'",
    "style-src 'self' 'unsafe-inline'",
    "connect-src 'self' https: wss: http://localhost:* http://127.0.0.1:*",
    "img-src 'self' data: https:",
    "font-src 'self'",
  ].join('; ')
}

function assertProductionBuildEnv(mode: string, env: Record<string, string>): void {
  if (mode !== 'production') return

  if (env.VITE_DEV_MNEMONIC?.trim()) {
    throw new Error(
      'VITE_DEV_MNEMONIC must not be set for production builds — it would be inlined into the client bundle. ' +
        'Remove it from .env, .env.local, .env.production, and your shell (GitLab #118, #378).'
    )
  }

  if (!env.VITE_WC_PROJECT_ID?.trim()) {
    throw new Error(
      'VITE_WC_PROJECT_ID must be set for production builds — a shared default project ID must not ship in the bundle (GitLab #378).'
    )
  }
}

function assertNonDevelopmentBuildMnemonic(mode: string, env: Record<string, string>): void {
  if (mode === 'development') return
  if (env.VITE_ALLOW_DEV_MNEMONIC === 'local-only') return
  if (!env.VITE_DEV_MNEMONIC?.trim()) return

  throw new Error(
    'VITE_DEV_MNEMONIC must not be set for non-development builds (mode=' +
      mode +
      '). Remove it from env files and the shell, or set VITE_ALLOW_DEV_MNEMONIC=local-only for intentional local-only bundles (GitLab #378).'
  )
}

function cspDevHosts(): Plugin {
  return {
    name: 'csp-dev-hosts',
    transformIndexHtml: {
      order: 'pre',
      handler(html, ctx) {
        if (!ctx.server) return html

        const ips: string[] = []
        for (const ifaces of Object.values(networkInterfaces())) {
          if (!ifaces) continue
          for (const { address, family, internal } of ifaces) {
            if (internal) continue
            if (!isPrivateIP(address)) {
              ctx.server.config.logger.warn(
                `\x1b[33m⚠ Public IP detected (${address}). Do not run vite on a public VPS.\x1b[0m`
              )
              continue
            }
            const host = family === 'IPv6' || (family as unknown) === 6 ? `[${address}]` : address
            ips.push(`http://${host}:*`)
          }
        }

        if (ips.length === 0) return html

        return html.replace('http://127.0.0.1:*;', `http://127.0.0.1:* ${ips.join(' ')};`)
      },
    },
  }
}

function cspProductionBuild(mode: string): Plugin {
  return {
    name: 'csp-production-build',
    transformIndexHtml: {
      order: 'pre',
      handler(html, ctx) {
        if (ctx.server) {
          return html.replace(/content="default-src[^"]*"/, `content="${buildDevCsp()}"`)
        }

        const env = loadEnv(mode, path.join(__dirname), 'VITE_')
        const prodCsp = buildProductionCsp(env)
        return html.replace(/content="default-src[^"]*"/, `content="${prodCsp}"`)
      },
    },
  }
}

export default defineConfig(({ mode, command }) => {
  const env = loadEnv(mode, path.join(__dirname), 'VITE_')

  if (command === 'build') {
    assertNonDevelopmentBuildMnemonic(mode, env)
    assertProductionBuildEnv(mode, env)
  }

  return {
    plugins: [react(), cspDevHosts(), cspProductionBuild(mode)],
    resolve: {
      dedupe: ['react', 'react-dom'],
      alias: {
        '@': path.resolve(__dirname, './src'),
        buffer: 'buffer',
        process: 'process/browser',
        util: 'util',
        stream: 'stream-browserify',
      },
    },
    define: {
      global: 'globalThis',
      'process.env': '{}',
      __GIT_SHA__: JSON.stringify(gitSha),
      __APP_VERSION__: JSON.stringify('v0.1.0'),
    },
    build: {
      outDir: 'dist',
      // Production bundles must not ship browser-facing source maps (GitLab #117).
      // Non-production `vite build --mode …` keeps maps for staging/debug pipelines.
      sourcemap: mode !== 'production',
      rollupOptions: {
        output: {
          manualChunks: (id) => {
            if (
              id.includes('node_modules/react') ||
              id.includes('node_modules/react-dom') ||
              id.includes('node_modules/scheduler')
            ) {
              return 'vendor-react'
            }
            if (
              id.includes('@goblinhunt/cosmes') ||
              id.includes('cosmjs') ||
              id.includes('bip39') ||
              id.includes('bip32')
            ) {
              return 'wallet-terra'
            }
            if (id.includes('@tanstack') || id.includes('zustand')) {
              return 'vendor-state'
            }
            if (id.includes('secp256k1') || id.includes('noble') || id.includes('scure') || id.includes('elliptic')) {
              return 'crypto'
            }
          },
        },
      },
      chunkSizeWarningLimit: 6000,
    },
    server: {
      port: 3000,
      open: true,
    },
    optimizeDeps: {
      esbuildOptions: {
        define: {
          global: 'globalThis',
        },
      },
    },
  }
})

// Exported for unit tests (GitLab #378).
export { assertNonDevelopmentBuildMnemonic, assertProductionBuildEnv, buildProductionCsp, buildDevCsp, originFromUrl }
