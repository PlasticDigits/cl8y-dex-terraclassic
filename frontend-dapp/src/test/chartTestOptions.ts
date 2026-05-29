import type { ChartOptions } from 'lightweight-charts'

/** Chart options shared by real-library Vitest suites (GitLab #211). */
export function baseRealChartOptions(width: number, height: number): ChartOptions {
  return {
    layout: {
      background: { type: 'solid', color: '#131722' },
      textColor: '#9ca3af',
      attributionLogo: false,
      panes: {
        enableResize: false,
        separatorColor: 'rgba(255,255,255,0.32)',
        separatorHoverColor: 'rgba(255,255,255,0.2)',
      },
    },
    grid: {
      vertLines: { color: 'rgba(255,255,255,0.04)' },
      horzLines: { color: 'rgba(255,255,255,0.04)' },
    },
    rightPriceScale: { borderColor: 'rgba(255,255,255,0.1)' },
    timeScale: {
      borderColor: 'rgba(255,255,255,0.1)',
      timeVisible: true,
      secondsVisible: false,
    },
    width,
    height,
  }
}
