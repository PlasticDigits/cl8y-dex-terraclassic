/** Buy (bid) / Sell (ask) chrome — semantic fills, not `tab-glass*` blue (GitLab #563). */
export function limitSideControlClass(opts: { compact?: boolean; tone: 'buy' | 'sell'; selected: boolean }): string {
  const size = opts.compact ? 'side-control side-control-compact' : 'side-control'
  const toneClass =
    opts.tone === 'buy'
      ? opts.selected
        ? 'side-buy-selected'
        : 'side-buy-idle'
      : opts.selected
        ? 'side-sell-selected'
        : 'side-sell-idle'
  return `${size} ${toneClass}`
}
