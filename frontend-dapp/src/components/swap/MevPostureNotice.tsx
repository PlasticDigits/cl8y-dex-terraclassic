import {
  MEV_POSTURE_DOCS_PATH,
  MEV_POSTURE_HEADING,
  MEV_POSTURE_NO_OPT_IN,
  MEV_POSTURE_SLIPPAGE_NOTE,
  MEV_POSTURE_SUMMARY,
} from '@/utils/mevPosture'

/**
 * Informational MEV / submission posture for Swap Settings (GitLab #168).
 * Disclosure only — no toggle until a real protected path exists.
 */
export function MevPostureNotice({ slippageTolerancePct }: { slippageTolerancePct: number }) {
  return (
    <div
      className="mb-4 sm:mb-6 card-neo animate-fade-in-up"
      data-testid="swap-mev-posture-notice"
      role="note"
      aria-labelledby="swap-mev-posture-heading"
    >
      <p id="swap-mev-posture-heading" className="label-neo mb-2">
        {MEV_POSTURE_HEADING}
      </p>
      <p className="text-[10px] mb-2 leading-relaxed" style={{ color: 'var(--ink-dim)' }}>
        {MEV_POSTURE_SUMMARY}
      </p>
      <p className="text-[10px] mb-2 leading-relaxed" style={{ color: 'var(--ink-dim)' }}>
        {MEV_POSTURE_SLIPPAGE_NOTE} <span className="font-mono">({slippageTolerancePct}%</span> active in Settings).
      </p>
      <p className="text-[10px] mb-2 leading-relaxed" style={{ color: 'var(--ink-subtle)' }}>
        {MEV_POSTURE_NO_OPT_IN}
      </p>
      <a
        href={MEV_POSTURE_DOCS_PATH}
        target="_blank"
        rel="noopener noreferrer"
        className="text-[10px] font-mono underline"
        style={{ color: 'var(--ink-subtle)' }}
      >
        docs/frontend.md#swap-mev-posture
      </a>
    </div>
  )
}
