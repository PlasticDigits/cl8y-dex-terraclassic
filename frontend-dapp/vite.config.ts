import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'
import { execSync } from 'child_process'

let gitSha = 'dev'
try {
  gitSha = execSync('git rev-parse --short HEAD').toString().trim()
} catch { /* not in git repo yet */ }

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      buffer: 'buffer',
      process: 'process/browser',
      util: 'util',
      stream: 'stream-browserify',
    },
  },
  define: {
    'global': 'globalThis',
    'process.env': '{}',
    __GIT_SHA__: JSON.stringify(gitSha),
    __APP_VERSION__: JSON.stringify('v0.1.0'),
  },
  build: {
    outDir: 'dist',
    sourcemap: true,
    rollupOptions: {
      output: {
        manualChunks: (id) => {
          if (id.includes('node_modules/react') ||
              id.includes('node_modules/react-dom') ||
              id.includes('node_modules/scheduler')) {
            return 'vendor-react'
          }
          if (id.includes('@goblinhunt/cosmes') ||
              id.includes('cosmjs') ||
              id.includes('bip39') ||
              id.includes('bip32')) {
            return 'wallet-terra'
          }
          if (id.includes('@tanstack') ||
              id.includes('zustand')) {
            return 'vendor-state'
          }
          if (id.includes('secp256k1') ||
              id.includes('noble') ||
              id.includes('scure') ||
              id.includes('elliptic')) {
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
})
