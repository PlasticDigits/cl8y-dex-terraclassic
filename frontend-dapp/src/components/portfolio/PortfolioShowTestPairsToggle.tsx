export type PortfolioShowTestPairsToggleProps = {
  checked: boolean
  onChange: (next: boolean) => void
  testPairCount: number
}

/** Short #534-style hatch so QA can still see gem P&L (GitLab #674, P674-3). */
export function PortfolioShowTestPairsToggle({ checked, onChange, testPairCount }: PortfolioShowTestPairsToggleProps) {
  const label = !checked && testPairCount > 0 ? `Show test pairs (${testPairCount})` : 'Show test pairs'
  return (
    <label
      className="flex items-center gap-2 text-xs cursor-pointer"
      style={{ color: 'var(--ink-dim)' }}
      data-testid="portfolio-show-test-pairs"
    >
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        data-testid="portfolio-show-test-pairs-input"
      />
      {label}
    </label>
  )
}
