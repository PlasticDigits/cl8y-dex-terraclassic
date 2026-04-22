import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import { resolve } from 'path'

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/lightweightChartsJsdomMock.ts', './src/test/setup.ts'],
    include: ['src/**/*.integration.test.{ts,tsx}'],
    exclude: ['node_modules/', 'dist/'],
    testTimeout: 120_000,
    hookTimeout: 60_000,
    env: {
      VITE_NETWORK: 'local',
      VITE_INDEXER_URL: process.env.VITE_INDEXER_URL ?? 'http://127.0.0.1:3001',
    },
  },
  resolve: {
    alias: {
      '@': resolve(__dirname, './src'),
    },
  },
})
