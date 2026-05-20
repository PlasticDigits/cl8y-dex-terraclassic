import { RetryError } from '@/components/ui'
import { LCD_CONNECTIVITY_OUTAGE_MESSAGE } from '@/utils/lcdConnectivity'

export interface LcdConnectivityBannerProps {
  onRetry: () => void
  /** When true, show a subtle "checking connection…" line under the banner. */
  isProbing?: boolean
}

/**
 * Global banner when the Terra LCD probe reports unreachable ([GitLab #171](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/171)).
 */
export function LcdConnectivityBanner({ onRetry, isProbing }: LcdConnectivityBannerProps) {
  return (
    <div className="alert-warning mb-4" role="alert" data-testid="lcd-connectivity-banner">
      <RetryError message={LCD_CONNECTIVITY_OUTAGE_MESSAGE} onRetry={onRetry} />
      <p className="text-center text-[11px] uppercase tracking-wide -mt-4 mb-2" style={{ color: 'var(--ink-subtle)' }}>
        {isProbing ? 'Checking connection…' : 'Reconnecting automatically every few seconds'}
      </p>
    </div>
  )
}
