import type { ReactNode } from 'react'
import { TxResultAlert } from '@/components/ui'
import { TerraBroadcastPendingLink } from '@/components/ui/TerraBroadcastPendingLink'
import type { TerraBroadcastPhase } from '@/services/terraclassic/terraBroadcast'
import { TRADE_MONEY_CTA_CLASS } from '@/utils/tradeMoneyCta'

/** Shared `/trade` ticket money-CTA chrome (GitLab #527). */
export const TRADE_TICKET_SUBMIT_FOOTER_TESTID = 'trade-ticket-submit-footer'

export type TradeMarketSubmitChromeModel = {
  canSubmit: boolean
  label: string
  onClick: () => void
  phase: TerraBroadcastPhase | null
  pendingTxHash: string | null
  isError: boolean
  errorMessage: string | null
  isSuccess: boolean
  successTxHash: string | undefined
}

/**
 * Opaque shrink-0 footer sibling of `.trade-order-ticket-scroll`.
 * Holds CTA + pending link + tx alerts only — never place/gas/crossing guards (T527-4).
 */
export function TradeTicketSubmitFooter({ children }: { children: ReactNode }) {
  return (
    <div className="trade-ticket-submit-footer" data-testid={TRADE_TICKET_SUBMIT_FOOTER_TESTID}>
      {children}
    </div>
  )
}

/** Market money CTA + broadcast chrome. Used inline (standalone tests) or in the ticket footer. */
export function TradeMarketSubmitChrome({ model }: { model: TradeMarketSubmitChromeModel }) {
  return (
    <>
      <button
        type="button"
        className={TRADE_MONEY_CTA_CLASS}
        disabled={!model.canSubmit}
        data-testid="trade-market-submit"
        onClick={model.onClick}
      >
        {model.label}
      </button>
      <TerraBroadcastPendingLink phase={model.phase} txHash={model.pendingTxHash} />
      {model.isError && model.errorMessage != null && <TxResultAlert type="error" message={model.errorMessage} />}
      {model.isSuccess && (
        <TxResultAlert type="success" message="Market swap confirmed." txHash={model.successTxHash} />
      )}
    </>
  )
}
