/** Thin wrapper so Vitest can defer `import('lightweight-charts')` (GitLab #225). */
export function loadPriceChartLightweightModule() {
  return import('lightweight-charts')
}
