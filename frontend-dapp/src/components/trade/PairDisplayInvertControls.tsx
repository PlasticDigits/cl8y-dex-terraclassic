/** Pair-orientation invert chrome (GitLab #524). Not the bid/ask side flip. */

export function PairDisplayInvertPill({
  label,
  ariaLabel,
  onToggle,
}: {
  label: string
  ariaLabel: string
  onToggle: () => void
}) {
  return (
    <button
      type="button"
      data-testid="trade-pair-invert-pill"
      aria-label={ariaLabel}
      onClick={onToggle}
      className="tab-glass !text-[10px] !px-2 !py-1 tab-glass-inactive font-semibold tracking-wide"
    >
      {label}
    </button>
  )
}

export function PairDisplayInvertIconButton({ ariaLabel, onToggle }: { ariaLabel: string; onToggle: () => void }) {
  return (
    <button
      type="button"
      data-testid="trade-ticket-pair-invert"
      aria-label={ariaLabel}
      onClick={onToggle}
      className="tab-glass !p-1.5 tab-glass-inactive shrink-0"
    >
      <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden>
        <path
          d="M2 6h10M12 6l-3-3M12 6l-3 3M14 10H4M4 10l3-3M4 10l3 3"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
    </button>
  )
}
