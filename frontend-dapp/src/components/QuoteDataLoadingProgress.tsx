/** Unknown publication duration stays indeterminate; progress never promises a quote or an ETA. */
export function QuoteDataLoadingProgress({ testId }: { testId: string }) {
  return (
    <div role="status" aria-live="polite" aria-atomic="true" className="space-y-1 text-xs" data-testid={testId}>
      <span>Loading price…</span>
      <progress aria-label="Loading price" aria-busy="true" className="block h-1 w-full" />
    </div>
  )
}
