import type { UseMutationResult } from '@tanstack/react-query'
import { TxResultAlert, Spinner } from '@/components/ui'
import { useLimitExpiredClaimMutation } from '@/hooks/useLimitExpiredClaimMutation'
import type { LimitOrderCancelInput } from '@/hooks/useLimitOrderCancelMutation'
import type { IndexerLimitCancellation, IndexerLimitPlacement, PairInfo } from '@/types'
import { getTokenDisplaySymbol } from '@/utils/tokenDisplay'
import { tradeDirectionSideLabels } from '@/utils/tradeDirectionSideLabels'
import { orderIdHasIndexedCancellation } from '@/utils/limitOrderCancelUserMessage'
import {
  chunkExpiredClaimOrderIds,
  confirmExpiredClaimBatchMessage,
  isOrderIdInExpiredClaimVariables,
  normalizeExpiredClaimOrderIds,
} from '@/utils/limitExpiredClaimBatch'
import {
  escrowTokenAddressForLimitSide,
  formatRemainingEscrowHuman,
  parkedClaimButtonLabel,
  partitionLimitPlacementsByLifecycle,
  partitionParkedPlacementsByKind,
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
  /** When true, disable cancel without pair-paused copy (e.g. trading blacklist). */
  cancelDisabled?: boolean
  openWalletModal: () => void
  /** Shared cancel mutation for one-click row cancel (#162, #419). */
  cancelLimitOrderMutation?: UseMutationResult<string, Error, LimitOrderCancelInput, unknown>
  /** Indexed cancellations for this pair — disables cancel on already-cancelled ids. */
  cancellations?: IndexerLimitCancellation[]
  /** When set, the active row for this `order_id` is visually emphasized (trade ticket "View order" — GitLab #161). */
  highlightOrderId?: number | null
}

function baseSymbolForPair(pair: PairInfo | undefined): string {
  if (!pair) return '—'
  const addr = pair.asset_infos[0]?.token?.contract_addr
  if (addr?.startsWith('terra1')) return getTokenDisplaySymbol(addr)
  return 'Base'
}

function sideRowLabel(side: string | null | undefined, baseSymbol: string): string {
  if (side === 'bid') return tradeDirectionSideLabels(baseSymbol).bidLabel
  if (side === 'ask') return tradeDirectionSideLabels(baseSymbol).askLabel
  return side ?? '?'
}

function isOrderIdInCancelVariables(orderId: number, variables: LimitOrderCancelInput | undefined): boolean {
  if (variables == null) return false
  if (Array.isArray(variables)) return variables.includes(orderId)
  return variables === orderId
}

function ParkedClaimRow({
  row,
  pair,
  dtPrefix,
  rowClass,
  compact,
  isWalletConnected,
  isPairPaused,
  claimsDisabled,
  claimMutation,
  openWalletModal,
}: {
  row: IndexerLimitPlacement
  pair: PairInfo | undefined
  dtPrefix: string
  rowClass: string
  compact: boolean
  isWalletConnected: boolean
  isPairPaused: boolean
  claimsDisabled: boolean
  claimMutation: ReturnType<typeof useLimitExpiredClaimMutation>
  openWalletModal: () => void
}) {
  const escrowAddr = escrowTokenAddressForLimitSide(pair, row.side)
  const sym = escrowAddr.startsWith('terra1') ? getTokenDisplaySymbol(escrowAddr) : 'escrow'
  const rem = formatRemainingEscrowHuman(row, pair)
  const claimLabel = parkedClaimButtonLabel(row)
  const isDust = claimLabel === 'Claim dust'
  const claiming = claimMutation.isPending && isOrderIdInExpiredClaimVariables(row.order_id, claimMutation.variables)
  const claimsBlocked = isPairPaused || claimsDisabled
  const claimDisabled = !isWalletConnected || claiming || claimsBlocked || claimMutation.isPending

  return (
    <li
      key={row.id}
      data-testid={`${dtPrefix}-placement-parked-${row.order_id}`}
      className={`rounded-md border-l-4 ${isDust ? 'border-slate-400/60 bg-white/[0.04]' : 'border-amber-500/70 bg-white/[0.04]'} px-2 py-2 space-y-2 ${rowClass}`}
    >
      <div>
        <span className={isDust ? 'text-slate-300/90 mr-1' : 'text-amber-400/95 mr-1'}>{isDust ? '▫' : '◆'}</span>
        order #{row.order_id} · {sideRowLabel(row.side, baseSymbolForPair(pair))} · ~{rem} {sym}
        {isDust ? (
          <span className="opacity-80"> · dust</span>
        ) : (
          row.parked_block_timestamp && (
            <span className="opacity-80"> · parked {row.parked_block_timestamp.slice(0, 19)}</span>
          )
        )}
      </div>
      <button
        type="button"
        data-testid={`${dtPrefix}-claim-expired-${row.order_id}`}
        className={
          compact ? 'btn-primary btn-cta w-full !text-[10px] !py-1' : 'btn-primary btn-cta w-full !text-xs !py-2'
        }
        disabled={claimDisabled}
        onClick={() => {
          if (!isWalletConnected) openWalletModal()
          else if (!claimsBlocked) claimMutation.mutate(row.order_id)
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
                : claimLabel}
      </button>
    </li>
  )
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
  cancelDisabled = false,
  openWalletModal,
  cancelLimitOrderMutation,
  cancellations = [],
  highlightOrderId = null,
}: LimitOrderMyPlacementsPanelProps) {
  const { active, parkedExpired } = partitionLimitPlacementsByLifecycle(rows)
  const { expired: expiredParked, dust: dustParked } = partitionParkedPlacementsByKind(parkedExpired)
  const claimMutation = useLimitExpiredClaimMutation(pairAddr, walletAddress || undefined)
  const compact = variant === 'compact'
  const baseSymbol = baseSymbolForPair(pair)
  const titleClass = compact
    ? 'uppercase tracking-wide font-semibold mb-1'
    : 'text-sm font-semibold uppercase tracking-wide'
  const rowClass = compact ? 'text-[10px] font-mono' : 'text-xs font-mono'
  const dtPrefix = compact ? 'trade' : 'limits-page'

  const claimsBlocked = isPairPaused || claimsDisabled
  const cancelBlocked = isPairPaused || cancelDisabled
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

  const onCancelActive = (orderId: number) => {
    if (!isWalletConnected) {
      openWalletModal()
      return
    }
    if (cancelBlocked || !cancelLimitOrderMutation) return
    if (orderIdHasIndexedCancellation(cancellations, orderId)) return
    const ok = window.confirm(`Cancel order #${orderId}?`)
    if (!ok) return
    cancelLimitOrderMutation.mutate(orderId)
  }

  const emptyCopy = 'No open limits.'

  const panelTitle = compact ? 'My open limits' : 'My open limits'

  return (
    <div className={compact ? 'space-y-1 border-t border-white/10 pt-3 max-h-48 overflow-y-auto' : 'space-y-2'}>
      <h2 className={titleClass} style={{ color: 'var(--ink-dim)' }}>
        {panelTitle}
      </h2>
      {isPairPaused && rows.length > 0 && (
        <p className="text-[10px] leading-snug alert-error !py-2 !px-2.5" role="status">
          Pair paused.
        </p>
      )}
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
                  Open
                </div>
              )}
              <ul className={`space-y-1.5 ${compact ? 'max-h-24 overflow-y-auto' : 'max-h-48 overflow-y-auto'}`}>
                {active.map((r) => {
                  const alreadyCancelled = orderIdHasIndexedCancellation(cancellations, r.order_id)
                  const pendingCancel =
                    cancelLimitOrderMutation?.isPending &&
                    isOrderIdInCancelVariables(r.order_id, cancelLimitOrderMutation.variables)
                  const cancelBtnDisabled =
                    !cancelLimitOrderMutation ||
                    !isWalletConnected ||
                    cancelBlocked ||
                    alreadyCancelled ||
                    pendingCancel ||
                    (cancelLimitOrderMutation?.isPending ?? false)
                  return (
                    <li
                      key={r.id}
                      data-testid={`${dtPrefix}-placement-active-${r.order_id}`}
                      className={`rounded-md border px-2 py-1.5 space-y-1.5 ${rowClass} transition-shadow duration-300 ${
                        highlightOrderId != null && r.order_id === highlightOrderId
                          ? 'border-amber-400/70 bg-white/[0.05] shadow-[0_0_0_2px_rgba(251,191,36,0.45)]'
                          : 'border-white/10 bg-white/[0.03]'
                      }`}
                    >
                      <div>
                        <span className="text-emerald-400/90 mr-1">●</span>
                        order #{r.order_id} · {sideRowLabel(r.side, baseSymbol)} · {r.price ?? '?'} · placed{' '}
                        {r.block_timestamp.slice(0, 19)}
                      </div>
                      {cancelLimitOrderMutation && (
                        <button
                          type="button"
                          data-testid={`${dtPrefix}-cancel-placement-${r.order_id}`}
                          className={
                            compact
                              ? 'rounded-lg border border-white/15 px-2 py-0.5 text-[9px] font-semibold uppercase tracking-wide hover:bg-white/5 disabled:opacity-40 w-full'
                              : 'rounded-lg border border-white/15 px-2.5 py-1 text-[10px] font-semibold uppercase tracking-wide hover:bg-white/5 disabled:opacity-40 w-full'
                          }
                          style={{ color: 'var(--ink-dim)' }}
                          disabled={cancelBtnDisabled}
                          onClick={() => onCancelActive(r.order_id)}
                        >
                          {!isWalletConnected
                            ? 'Connect to cancel'
                            : isPairPaused
                              ? 'Unavailable (pair paused)'
                              : cancelDisabled
                                ? 'Trading restricted'
                                : pendingCancel
                                  ? 'Cancelling…'
                                  : 'Cancel'}
                        </button>
                      )}
                    </li>
                  )
                })}
              </ul>
            </div>
          )}
          {expiredParked.length > 0 && (
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
                        ? 'rounded-lg border border-amber-500/40 px-2 py-0.5 text-[9px] font-semibold uppercase tracking-wide hover:bg-white/5 disabled:opacity-40'
                        : 'rounded-lg border border-amber-500/40 px-2.5 py-1 text-[10px] font-semibold uppercase tracking-wide hover:bg-white/5 disabled:opacity-40'
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
                {expiredParked.map((r) => (
                  <ParkedClaimRow
                    key={r.id}
                    row={r}
                    pair={pair}
                    dtPrefix={dtPrefix}
                    rowClass={rowClass}
                    compact={compact}
                    isWalletConnected={isWalletConnected}
                    isPairPaused={isPairPaused}
                    claimsDisabled={claimsDisabled}
                    claimMutation={claimMutation}
                    openWalletModal={openWalletModal}
                  />
                ))}
              </ul>
            </div>
          )}
          {dustParked.length > 0 && (
            <div className="space-y-1">
              <div
                className={
                  compact
                    ? 'text-[10px] uppercase tracking-wide font-medium'
                    : 'text-[11px] uppercase tracking-wide font-medium'
                }
                style={{ color: 'var(--ink-dim)' }}
              >
                Dust — claim remaining
              </div>
              <ul className="space-y-2">
                {dustParked.map((r) => (
                  <ParkedClaimRow
                    key={r.id}
                    row={r}
                    pair={pair}
                    dtPrefix={dtPrefix}
                    rowClass={rowClass}
                    compact={compact}
                    isWalletConnected={isWalletConnected}
                    isPairPaused={isPairPaused}
                    claimsDisabled={claimsDisabled}
                    claimMutation={claimMutation}
                    openWalletModal={openWalletModal}
                  />
                ))}
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
      {cancelLimitOrderMutation?.isError && (
        <TxResultAlert type="error" message={(cancelLimitOrderMutation.error as Error).message} />
      )}
      {cancelLimitOrderMutation?.isSuccess && !cancelLimitOrderMutation.isPending && (
        <TxResultAlert type="success" message="Cancel submitted." txHash={cancelLimitOrderMutation.data} />
      )}
    </div>
  )
}
