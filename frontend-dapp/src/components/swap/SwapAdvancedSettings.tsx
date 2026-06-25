import type { UseQueryResult } from '@tanstack/react-query'
import type { PairInfo } from '@/types'
import type { IndexerRouteSolveResponse } from '@/types'
import { AmountBalanceActions } from '@/components/common/AmountBalanceActions'
import { pairInfoMenuLabel } from '@/utils/pairMenuOptions'
import { getTokenDisplaySymbol, shortenAddress } from '@/utils/tokenDisplay'
import { isDecimalAmountDraft } from '@/utils/decimalAmountInput'
import { sounds } from '@/lib/sounds'

type Props = {
  open: boolean
  onOpenChange: (open: boolean) => void
  isDirect: boolean
  isWrapOrUnwrap: boolean
  directPair: PairInfo | null | undefined
  fromToken: string
  toToken: string
  useHybridBook: boolean
  onUseHybridBookChange: (enabled: boolean) => void
  bookInputHuman: string
  onBookInputHumanChange: (value: string) => void
  hybridMaxMakers: number
  onHybridMaxMakersChange: (value: number) => void
  bookLegAmountInputId: string
  hybridMaxMakersInputId: string
  isWalletConnected: boolean
  balanceQuery: UseQueryResult<string>
  offerDecimals: number
  bookLegMaxResult: { spendableRaw: string; human: string }
  onCheckIndexerRoute: () => void
  indexerRouteLoading: boolean
  indexerRouteError: string | null
  indexerRouteResult: IndexerRouteSolveResponse | null
  clientRouteHopCount: number | null
}

/**
 * Collapsible integrator controls: direct-pair hybrid book leg and indexer route debug.
 * Collapsed by default (GitLab #413).
 */
export function SwapAdvancedSettings({
  open,
  onOpenChange,
  isDirect,
  isWrapOrUnwrap,
  directPair,
  fromToken,
  toToken,
  useHybridBook,
  onUseHybridBookChange,
  bookInputHuman,
  onBookInputHumanChange,
  hybridMaxMakers,
  onHybridMaxMakersChange,
  bookLegAmountInputId,
  hybridMaxMakersInputId,
  isWalletConnected,
  balanceQuery,
  offerDecimals,
  bookLegMaxResult,
  onCheckIndexerRoute,
  indexerRouteLoading,
  indexerRouteError,
  indexerRouteResult,
  clientRouteHopCount,
}: Props) {
  const showHybridBook = isDirect && !isWrapOrUnwrap && !!directPair

  return (
    <div
      id="swap-advanced-settings"
      className="mb-4 sm:mb-6 card-glass animate-fade-in-up"
      data-testid="swap-advanced-settings"
    >
      <details open={open} data-testid="swap-advanced-settings-details">
        <summary
          className="cursor-pointer text-sm font-semibold uppercase tracking-wide"
          style={{ color: 'var(--cyan)' }}
          data-testid="swap-advanced-settings-toggle"
          onClick={(e) => {
            e.preventDefault()
            onOpenChange(!open)
          }}
        >
          Advanced
        </summary>
        <div className="mt-3 space-y-4 border-t pt-3" style={{ borderColor: 'var(--border-subtle)' }}>
          {showHybridBook && directPair && (
            <div data-testid="swap-hybrid-book-settings">
              <p className="label-glass mb-2">Limit book leg</p>
              <p className="text-[10px] font-mono mb-2" style={{ color: 'var(--ink-subtle)' }}>
                {pairInfoMenuLabel(directPair, { variant: 'full' })}
              </p>
              <p className="text-[10px] mb-3 leading-relaxed" style={{ color: 'var(--ink-dim)' }}>
                For single-hop CW20 swaps only. Route part of your payment through resting limit orders; quotes use the
                same hybrid simulation as submit.
              </p>
              <label className="flex items-center gap-2 text-xs mb-3 cursor-pointer">
                <input
                  type="checkbox"
                  checked={useHybridBook}
                  onChange={(e) => onUseHybridBookChange(e.target.checked)}
                />
                Route part of input through the limit book
              </label>
              {useHybridBook && (
                <div className="space-y-2">
                  <div>
                    <label className="label-glass text-[10px]" htmlFor={bookLegAmountInputId}>
                      Book leg amount ({getTokenDisplaySymbol(fromToken)})
                    </label>
                    <input
                      id={bookLegAmountInputId}
                      type="text"
                      inputMode="decimal"
                      className="input-glass !text-xs w-full"
                      value={bookInputHuman}
                      onChange={(e) => {
                        const v = e.target.value
                        if (isDecimalAmountDraft(v)) onBookInputHumanChange(v)
                      }}
                      placeholder="0.0"
                    />
                    {isWalletConnected && fromToken.startsWith('terra1') && (
                      <AmountBalanceActions
                        balanceQuery={balanceQuery}
                        decimals={offerDecimals}
                        walletConnected={isWalletConnected}
                        compact
                        spendableRaw={bookLegMaxResult.spendableRaw}
                        onMax={() => onBookInputHumanChange(bookLegMaxResult.human)}
                        testIdMax="swap-book-leg-max"
                      />
                    )}
                  </div>
                  <div>
                    <label className="label-glass text-[10px]" htmlFor={hybridMaxMakersInputId}>
                      Max distinct makers
                    </label>
                    <input
                      id={hybridMaxMakersInputId}
                      type="number"
                      className="input-glass !text-xs w-full"
                      min={1}
                      max={256}
                      value={hybridMaxMakers}
                      onChange={(e) => onHybridMaxMakersChange(Number(e.target.value) || 8)}
                    />
                  </div>
                </div>
              )}
            </div>
          )}

          <div data-testid="swap-indexer-route-check">
            <p className="label-glass mb-3">Indexer route check</p>
            <p className="text-[10px] mb-3 leading-relaxed" style={{ color: 'var(--ink-dim)' }}>
              Compares this token pair with the indexer&apos;s BFS graph (max 4 hops). Only CW20 addresses present in
              the indexer asset table are supported; native-only assets without a CW20 row are not routable via{' '}
              <code className="font-mono text-[10px]">/api/v1/route/solve</code>.
            </p>
            <button
              type="button"
              className="btn-muted !text-xs"
              onClick={() => {
                sounds.playButtonPress()
                onCheckIndexerRoute()
              }}
              disabled={indexerRouteLoading || !fromToken || !toToken}
            >
              {indexerRouteLoading ? 'Checking…' : 'Compare indexer route'}
            </button>
            {indexerRouteError && (
              <p className="text-xs mt-2 font-medium" style={{ color: 'var(--color-negative)' }}>
                {indexerRouteError}
              </p>
            )}
            {indexerRouteResult && (
              <div className="mt-3 text-[11px] space-y-1.5 font-mono" style={{ color: 'var(--ink-subtle)' }}>
                <p>
                  Indexer hops: {indexerRouteResult.hops.length}
                  {clientRouteHopCount != null && (
                    <span style={{ color: 'var(--ink-dim)' }}> · Client hops: {clientRouteHopCount}</span>
                  )}
                </p>
                {indexerRouteResult.hops.map((h, i) => (
                  <p key={`${h.pair}-${i}`}>
                    {i + 1}. {shortenAddress(h.pair, 8, 6)} · {shortenAddress(h.offer_token, 4, 4)} →{' '}
                    {shortenAddress(h.ask_token, 4, 4)}
                  </p>
                ))}
              </div>
            )}
          </div>
        </div>
      </details>
    </div>
  )
}
