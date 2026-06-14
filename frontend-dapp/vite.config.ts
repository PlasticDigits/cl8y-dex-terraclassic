import { defineConfig, loadEnv } from 'vite'
import type { Plugin } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'
import { execSync } from 'child_process'
import { networkInterfaces } from 'os'
import { buildProductionCspMetaContent } from './viteCsp'

let gitSha = 'dev'
try {
  gitSha = execSync('git rev-parse --short HEAD').toString().trim()
} catch {
  /* not in git repo yet */
}

function isPrivateIP(addr: string): boolean {
  return /^(10\.|172\.(1[6-9]|2\d|3[01])\.|192\.168\.|fd[0-9a-f]{2}:|fe80:)/.test(addr)
}

function assertBuildEnvGuards(command: string, mode: string, env: Record<string, string>): void {
  if (command !== 'build') return

  const mnemonic = env.VITE_DEV_MNEMONIC?.trim()
  if (mnemonic) {
    const allowMnemonic = mode === 'development' || env.VITE_ALLOW_DEV_MNEMONIC === 'local-only'
    if (!allowMnemonic) {
      throw new Error(
        'VITE_DEV_MNEMONIC must not be set for non-development vite builds unless VITE_ALLOW_DEV_MNEMONIC=local-only. ' +
          'Remove it from .env, .env.local, .env.staging, and your shell (GitLab #378 / #118).'
      )
    }
  }

  if (mode === 'production' && !env.VITE_WC_PROJECT_ID?.trim()) {
    throw new Error(
      'VITE_WC_PROJECT_ID is required for production builds. Set it in .env.production or your CI/CD secret store (GitLab #378 / M-10).'
    )
  }
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

function cspProductionPolicy(mode: string, env: Record<string, string>): Plugin {
  return {
    name: 'csp-production-policy',
    transformIndexHtml: {
      order: 'pre',
      handler(html) {
        if (mode !== 'production') return html
        const csp = buildProductionCspMetaContent(env)
        return html.replace(
          /<meta http-equiv="Content-Security-Policy"\s+content="[^"]*"\s*\/>/,
          `<meta http-equiv="Content-Security-Policy" content="${csp}" />`
        )
      },
    },
  }
}

export default defineConfig(({ mode, command }) => {
  const env = loadEnv(mode, path.join(__dirname), 'VITE_')
  assertBuildEnvGuards(command, mode, env)

  return {
    plugins: [react(), cspDevHosts(), cspProductionPolicy(mode, env)],
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
