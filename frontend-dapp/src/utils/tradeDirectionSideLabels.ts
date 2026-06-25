/** CEX-standard Buy/Sell labels for `/trade` bid/ask buttons (GitLab #300, #412). */
export function tradeDirectionSideLabels(baseSymbol: string): { bidLabel: string; askLabel: string } {
  return {
    bidLabel: `Buy ${baseSymbol}`,
    askLabel: `Sell ${baseSymbol}`,
  }
}
