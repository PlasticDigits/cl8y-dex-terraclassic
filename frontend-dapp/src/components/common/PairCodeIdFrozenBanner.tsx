import { USER_INCIDENT_FAQ_HREF } from '@/components/legal/legalCopy'
import { CODE_ID_FROZEN_BANNER } from '@/utils/assetCodeIdFreeze'

export function PairCodeIdFrozenBanner({ testId }: { testId: string }) {
  return (
    <div className="alert-error mb-3 text-xs space-y-2" role="alert" data-testid={testId}>
      <p>{CODE_ID_FROZEN_BANNER}</p>
      <a className="underline text-[10px]" href={USER_INCIDENT_FAQ_HREF} target="_blank" rel="noopener noreferrer">
        Docs
      </a>
    </div>
  )
}
