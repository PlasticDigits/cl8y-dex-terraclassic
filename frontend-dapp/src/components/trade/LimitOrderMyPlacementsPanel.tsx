import { useMutation, useQueryClient } from '@tanstack/react-query'
import { claimExpiredLimitOrder } from '@/services/terraclassic/pair'
import { TxResultAlert, Spinner } from '@/components/ui'
import { sounds } from '@/lib/sounds'
import type { IndexerLimitPlacement, PairInfo } from '@/types'
import { getTokenDisplaySymbol } from '@/utils/tokenDisplay'
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
  openWalletModal: () => void
}

export function LimitOrderMyPlacementsPanel({
  variant,
  pairAddr,
  pair,
  walletAddress,
  rows,
  isLoading,
  isWalletConnected,
  openWalletModal,
}: LimitOrderMyPlacementsPanelProps) {
  const queryClient = useQueryClient()
  const { active, parkedExpired } = partitionLimitPlacementsByLifecycle(rows)
  const compact = variant === 'compact'
  const titleClass = compact
    ? 'uppercase tracking-wide font-semibold mb-1'
    : 'text-sm font-semibold uppercase tracking-wide'
  const rowClass = compact ? 'text-[10px] font-mono' : 'text-xs font-mono'
  const dtPrefix = compact ? 'trade' : 'limits-page'

  const claimMutation = useMutation({
    mutationFn: async (orderId: number) => {
      if (!walletAddress) throw new Error('Connect wallet')
      return claimExpiredLimitOrder(walletAddress, pairAddr, orderId)
    },
    onSuccess: () => {
      sounds.playSuccess()
      queryClient.invalidateQueries({ queryKey: ['limitPlacements'] })
      queryClient.invalidateQueries({ queryKey: ['limitBookPagePreview', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['limitBookPage', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['tokenBalance'] })
    },
    onError: () => sounds.playError(),
  })

  const emptyCopy =
    'No indexed placements for this wallet on this pair (or pair code predates owner attrs). ' +
    'Expired limits removed during another trader’s swap can still appear here once the indexer marks them parked expired — use Claim refund to recover escrow.'

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
                    className={`rounded-md border border-white/10 bg-white/[0.03] px-2 py-1.5 ${rowClass}`}
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
              <ul className="space-y-2">
                {parkedExpired.map((r) => {
                  const escrowAddr = escrowTokenAddressForLimitSide(pair, r.side)
                  const sym = escrowAddr.startsWith('terra1') ? getTokenDisplaySymbol(escrowAddr) : 'escrow'
                  const rem = formatRemainingEscrowHuman(r, pair)
                  const claiming = claimMutation.isPending && claimMutation.variables === r.order_id
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
                        disabled={!isWalletConnected || claiming}
                        onClick={() => {
                          if (!isWalletConnected) openWalletModal()
                          else claimMutation.mutate(r.order_id)
                        }}
                      >
                        {!isWalletConnected ? 'Connect wallet to claim' : claiming ? 'Claiming…' : 'Claim refund'}
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
