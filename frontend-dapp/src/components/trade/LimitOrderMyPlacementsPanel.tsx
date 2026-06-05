import { TxResultAlert, Spinner } from '@/components/ui'
import { useLimitExpiredClaimMutation } from '@/hooks/useLimitExpiredClaimMutation'
import type { IndexerLimitPlacement, PairInfo } from '@/types'
import { getTokenDisplaySymbol } from '@/utils/tokenDisplay'
import {
  chunkExpiredClaimOrderIds,
  confirmExpiredClaimBatchMessage,
  isOrderIdInExpiredClaimVariables,
  normalizeExpiredClaimOrderIds,
} from '@/utils/limitExpiredClaimBatch'
import {
  escrowTokenAddressForLimitSide,
  formatRemainingEscrowHuman,
  partitionLimitPlacementsByLifecycle,
} from '@/utils/limitPlacementLifecycle'

export type LimitOrderMyPlacementsVariant = 'page' | 'compact'

export interface LimitOrderMyPlacementsPanelProps {
  variant: LimitOrderMyPlacementsVariant
  pairAddr: string
  pair: PairInfo | undefined
  walletAddress: string
  rows: IndexerLimitPlacement[]
  isLoading: boolean
  isWalletConnected: boolean
  /** When true, on-chain claim is rejected — disable Claim refund (L6 / GitLab #120). */
  isPairPaused: boolean
  /** When true, disable claims without pair-paused copy (e.g. trading blacklist). */
  claimsDisabled?: boolean
  openWalletModal: () => void
  /** When set, the active row for this `order_id` is visually emphasized (trade ticket "View order" — GitLab #161). */
  highlightOrderId?: number | null
}

export function LimitOrderMyPlacementsPanel({
  variant,
  pairAddr,
  pair,
  walletAddress,
  rows,
  isLoading,
  isWalletConnected,
  isPairPaused,
  claimsDisabled = false,
  openWalletModal,
  highlightOrderId = null,
}: LimitOrderMyPlacementsPanelProps) {
  const { active, parkedExpired } = partitionLimitPlacementsByLifecycle(rows)
  const claimMutation = useLimitExpiredClaimMutation(pairAddr, walletAddress || undefined)
  const compact = variant === 'compact'
  const titleClass = compact
    ? 'uppercase tracking-wide font-semibold mb-1'
    : 'text-sm font-semibold uppercase tracking-wide'
  const rowClass = compact ? 'text-[10px] font-mono' : 'text-xs font-mono'
  const dtPrefix = compact ? 'trade' : 'limits-page'

  const claimsBlocked = isPairPaused || claimsDisabled
  const claimAllDisabled = !isWalletConnected || claimsBlocked || claimMutation.isPending || parkedExpired.length < 2

  const onClaimAllParked = async () => {
    if (!isWalletConnected) {
      openWalletModal()
      return
    }
    if (claimsBlocked || parkedExpired.length < 2) return

    const orderIds = normalizeExpiredClaimOrderIds(parkedExpired.map((r) => r.order_id))
    const chunks = chunkExpiredClaimOrderIds(orderIds)

    try {
      for (let i = 0; i < chunks.length; i++) {
        const chunk = chunks[i]!
        const ok = window.confirm(confirmExpiredClaimBatchMessage(chunk, i, chunks.length, orderIds.length))
        if (!ok) return
        await claimMutation.mutateAsync(chunk)
      }
    } catch {
      // TxResultAlert surfaces mutation.error
    }
  }

  const emptyCopy =
    'No indexed placements for this wallet on this pair (or pair code predates owner attrs). ' +
    "Expired limits removed during another trader's swap can still appear here once the indexer marks them parked expired — use Claim refund to recover escrow."

  return (
    <div className={compact ? 'space-y-1 border-t border-white/10 pt-3 max-h-48 overflow-y-auto' : 'space-y-2'}>
      <h2 className={titleClass} style={{ color: 'var(--ink-dim)' }}>
        {compact ? 'My limits (indexer)' : 'My limits (indexer)'}
      </h2>
      {isLoading && <Spinner />}
      {!isLoading && rows.length === 0 && (
        <p className={compact ? 'text-[10px] opacity-90' : 'text-sm'} style={{ color: 'var(--ink-dim)' }}>
          {emptyCopy}
        </p>
      )}
      {!isLoading && rows.length > 0 && (
        <div className="space-y-3">
          {active.length > 0 && (
            <div className="space-y-1">
              {!compact && (
                <div className="text-[11px] uppercase tracking-wide font-medium" style={{ color: 'var(--ink-dim)' }}>
                  Active on book
                </div>
              )}
              <ul className={`space-y-1 ${compact ? 'max-h-24 overflow-y-auto' : 'max-h-40 overflow-y-auto'}`}>
                {active.map((r) => (
                  <li
                    key={r.id}
                    data-testid={`${dtPrefix}-placement-active-${r.order_id}`}
                    className={`rounded-md border px-2 py-1.5 ${rowClass} transition-shadow duration-300 ${
                      highlightOrderId != null && r.order_id === highlightOrderId
                        ? 'border-amber-400/70 bg-amber-500/[0.12] shadow-[0_0_0_2px_rgba(251,191,36,0.45)]'
                        : 'border-white/10 bg-white/[0.03]'
                    }`}
                  >
                    <span className="text-emerald-400/90 mr-1">●</span>
                    order #{r.order_id} · {r.side ?? '?'} · {r.price ?? '?'} · placed {r.block_timestamp.slice(0, 19)}
                  </li>
                ))}
              </ul>
            </div>
          )}
          {parkedExpired.length > 0 && (
            <div className="space-y-1">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div
                  className={
                    compact
                      ? 'text-[10px] uppercase tracking-wide font-medium'
                      : 'text-[11px] uppercase tracking-wide font-medium'
                  }
                  style={{ color: 'var(--ink-dim)' }}
                >
                  Expired — refund pending
                </div>
                {parkedExpired.length >= 2 && (
                  <button
                    type="button"
                    data-testid={`${dtPrefix}-claim-all-parked`}
                    className={
                      compact
                        ? 'rounded-lg border border-amber-500/40 px-2 py-0.5 text-[9px] font-semibold uppercase tracking-wide hover:bg-amber-500/10 disabled:opacity-40'
                        : 'rounded-lg border border-amber-500/40 px-2.5 py-1 text-[10px] font-semibold uppercase tracking-wide hover:bg-amber-500/10 disabled:opacity-40'
                    }
                    style={{ color: 'var(--ink-dim)' }}
                    disabled={claimAllDisabled}
                    title="Uses indexer parked-expired rows for your wallet on this pair."
                    onClick={() => void onClaimAllParked()}
                  >
                    {!isWalletConnected
                      ? 'Connect to claim all'
                      : isPairPaused
                        ? 'Unavailable (pair paused)'
                        : claimsDisabled
                          ? 'Trading restricted'
                          : claimMutation.isPending
                            ? 'Claiming…'
                            : `Claim all parked (${parkedExpired.length})`}
                  </button>
                )}
              </div>
              <ul className="space-y-2">
                {parkedExpired.map((r) => {
                  const escrowAddr = escrowTokenAddressForLimitSide(pair, r.side)
                  const sym = escrowAddr.startsWith('terra1') ? getTokenDisplaySymbol(escrowAddr) : 'escrow'
                  const rem = formatRemainingEscrowHuman(r, pair)
                  const claiming =
                    claimMutation.isPending && isOrderIdInExpiredClaimVariables(r.order_id, claimMutation.variables)
                  const claimDisabled = !isWalletConnected || claiming || claimsBlocked || claimMutation.isPending
                  return (
                    <li
                      key={r.id}
                      data-testid={`${dtPrefix}-placement-parked-${r.order_id}`}
                      className={`rounded-md border-l-4 border-amber-500/70 bg-amber-500/[0.06] px-2 py-2 space-y-2 ${rowClass}`}
                    >
                      <div>
                        <span className="text-amber-400/95 mr-1">◆</span>
                        order #{r.order_id} · {r.side ?? '?'} · ~{rem} {sym}
                        {r.parked_block_timestamp && (
                          <span className="opacity-80"> · parked {r.parked_block_timestamp.slice(0, 19)}</span>
                        )}
                      </div>
                      <button
                        type="button"
                        data-testid={`${dtPrefix}-claim-expired-${r.order_id}`}
                        className={
                          compact
                            ? 'btn-primary btn-cta w-full !text-[10px] !py-1'
                            : 'btn-primary btn-cta w-full !text-xs !py-2'
                        }
                        disabled={claimDisabled}
                        onClick={() => {
                          if (!isWalletConnected) openWalletModal()
                          else if (!claimsBlocked) claimMutation.mutate(r.order_id)
                        }}
                      >
                        {!isWalletConnected
                          ? 'Connect wallet to claim'
                          : isPairPaused
                            ? 'Unavailable (pair paused)'
                            : claimsDisabled
                              ? 'Trading restricted'
                              : claiming
                                ? 'Claiming…'
                                : 'Claim refund'}
                      </button>
                    </li>
                  )
                })}
              </ul>
            </div>
          )}
        </div>
      )}
      {claimMutation.isError && (
        <div data-testid={`${dtPrefix}-claim-result`}>
          <TxResultAlert type="error" message={(claimMutation.error as Error).message} />
        </div>
      )}
      {claimMutation.isSuccess && !claimMutation.isPending && (
        <div data-testid={`${dtPrefix}-claim-result`}>
          <TxResultAlert type="success" message="Refund transaction submitted." txHash={claimMutation.data} />
        </div>
      )}
    </div>
  )
}
