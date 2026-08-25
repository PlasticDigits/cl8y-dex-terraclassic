import { Link } from 'react-router-dom'
import {
  COMMUNITY_TAX_REGISTER_ALERT_COPY,
  registerLargestPoolLabel,
  type UnregisteredFactoryPair,
} from '@/utils/communityTaxRegisterPair'

export type ManageUnregisteredPairAlertProps = {
  target: UnregisteredFactoryPair
  leftover: number
  otherTokens: { address: string; symbol: string }[]
  pending?: boolean
  onRegister: () => void
}

/** Highly visible Manage catch-up (GitLab #633 / **R633-4**). One button. No URL prefill. */
export function ManageUnregisteredPairAlert({
  target,
  leftover,
  otherTokens,
  pending,
  onRegister,
}: ManageUnregisteredPairAlertProps) {
  return (
    <div className="alert-warning space-y-3" data-testid="manage-register-alert" role="alert">
      <p className="text-sm font-semibold" style={{ color: 'var(--ink)' }}>
        {COMMUNITY_TAX_REGISTER_ALERT_COPY}
      </p>
      <button
        type="button"
        className="btn-primary w-full"
        disabled={pending}
        onClick={onRegister}
        data-testid="manage-register-largest"
      >
        {pending ? 'Registering…' : registerLargestPoolLabel(target)}
      </button>
      {leftover > 0 && (
        <p className="text-xs" style={{ color: 'var(--ink-dim)' }} data-testid="manage-register-leftover">
          After this, {leftover} more {leftover === 1 ? 'pool still needs' : 'pools still need'} the same step.
        </p>
      )}
      {otherTokens.length > 0 && (
        <p className="text-xs" style={{ color: 'var(--ink-dim)' }} data-testid="manage-register-other">
          Another token you manage still needs a pool registered:{' '}
          {otherTokens.map((t, i) => (
            <span key={t.address}>
              {i > 0 ? ', ' : null}
              <Link className="underline" to={`/token/${t.address}/manage`}>
                {t.symbol}
              </Link>
            </span>
          ))}
        </p>
      )}
      <p className="text-xs" style={{ color: 'var(--ink-dim)' }}>
        <Link className="underline" to={`/pool/${target.pair}`}>
          View this pool
        </Link>
      </p>
    </div>
  )
}
