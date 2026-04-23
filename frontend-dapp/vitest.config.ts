import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import { resolve } from 'path'

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/lightweightChartsJsdomMock.ts', './src/test/setup.ts'],
    env: {
      VITE_NETWORK: 'local',
      /** Enables factory-backed queries in pages that gate on `FACTORY_CONTRACT_ADDRESS` (e.g. Pool list router badges). */
      VITE_FACTORY_ADDRESS: 'terra1f0000000000000000000000000000000000000',
    },
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
    exclude: ['node_modules/', 'dist/', 'src/**/*.integration.test.{ts,tsx}'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
    },
    testTimeout: 10000,
  },
  resolve: {
    alias: {
      '@': resolve(__dirname, './src'),
    },
  },
})
