import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import { resolve } from 'path'

/** Real TradingView lightweight-charts (open-source) — not the hosted widget. See GitLab #211. */
export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/chartsSetup.ts'],
    include: ['src/**/*.charts.test.{ts,tsx}'],
    exclude: ['node_modules/', 'dist/'],
    testTimeout: 15000,
    fileParallelism: false,
    environmentOptions: {
      jsdom: { resources: 'usable' },
    },
  },
  resolve: {
    alias: {
      '@': resolve(__dirname, './src'),
    },
  },
})
