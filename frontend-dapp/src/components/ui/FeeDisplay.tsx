export interface FeeDisplayProps {
  feeBps: number
  discountBps?: number
  commissionAmount?: string
}

export function FeeDisplay({ feeBps, discountBps = 0, commissionAmount }: FeeDisplayProps) {
  const baseFeePct = (feeBps / 100).toFixed(2)
  const effectiveFeePct = discountBps > 0 ? ((feeBps * (10000 - discountBps)) / 10000 / 100).toFixed(2) : baseFeePct
  const discountPctLabel = (discountBps / 100).toFixed(0)

  return (
    <span>
      {discountBps > 0 ? (
        <>
          <span className="line-through mr-1" style={{ color: 'var(--ink-subtle)' }}>
            {baseFeePct}%
          </span>
          <span style={{ color: 'var(--cyan)' }}>{effectiveFeePct}%</span>
          <span className="text-xs ml-1" style={{ color: 'var(--cyan)' }}>
            (-{discountPctLabel}%)
          </span>
        </>
      ) : (
        <>{baseFeePct}%</>
      )}
      {commissionAmount != null && (
        <span className="ml-1" style={{ color: 'var(--ink-subtle)' }}>
          ({commissionAmount})
        </span>
      )}
    </span>
  )
}
