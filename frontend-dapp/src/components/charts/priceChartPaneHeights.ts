/** Pane height helpers for TradingView lightweight-charts multi-pane layout (not the hosted widget). */

/** Histogram pane below candles — sized so volume bars stay readable. */
export function volumePaneHeightPx(totalChartHeight: number): number {
  return Math.min(88, Math.max(48, Math.round(totalChartHeight * 0.085)))
}

export function rsiPaneHeightPx(totalChartHeight: number): number {
  return Math.min(140, Math.max(72, Math.round(totalChartHeight * 0.17)))
}
