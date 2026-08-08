import { useMemo, useState } from 'react'
import { Link } from 'react-router-dom'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { useDebouncedValue } from '@/hooks/useDebouncedValue'
import { useTerraBroadcastMutation } from '@/hooks/useTerraBroadcastMutation'
import { useTokenBalance } from '@/hooks/useTokenBalance'
import { isUst1WindowEnabled, UST1_WINDOW_CONTRACT_ADDRESS } from '@/utils/constants'
import {
  executeUst1Window,
  getUst1EffectiveSwap,
  paySymbolForDirection,
  payTokenForDirection,
  receiveSymbolForDirection,
} from '@/services/terraclassic/ust1Window'
import { formatTokenAmountAbbrev, fromRawAmount, toRawAmount } from '@/utils/formatAmount'
import { isDecimalAmountDraft, isPositiveDecimalAmount, tryParseBigInt } from '@/utils/decimalAmountInput'
import {
  evaluateUst1SubmitGate,
  isOracleStale,
  rollingRemainingUst1,
  type Ust1WindowDirection,
} from '@/utils/ust1WindowGates'
import { Spinner, RetryError } from '@/components/ui'
import { TerraBroadcastPendingLink } from '@/components/ui/TerraBroadcastPendingLink'
import { sounds } from '@/lib/sounds'
import { humanizeUserFacingErrorFromUnknown } from '@/utils/humanizeUserFacingError'

const UST1_DECIMALS = 6
const QUOTE_DEBOUNCE_MS = 250

export default function Ust1Page() {
  const address = useWalletStore((s) => s.address)
  const queryClient = useQueryClient()
  const [direction, setDirection] = useState<Ust1WindowDirection>('deposit')
  const [payHuman, setPayHuman] = useState('')
  const [successTx, setSuccessTx] = useState<string | null>(null)
  const debouncedPayHuman = useDebouncedValue(payHuman, QUOTE_DEBOUNCE_MS)

  const windowEnabled = isUst1WindowEnabled()
  const payToken = payTokenForDirection(direction)
  const balanceQuery = useTokenBalance(address, payToken)

  const effectiveQuery = useQuery({
    queryKey: ['ust1EffectiveSwap', UST1_WINDOW_CONTRACT_ADDRESS],
    queryFn: getUst1EffectiveSwap,
    enabled: windowEnabled,
    staleTime: 15_000,
    refetchInterval: 30_000,
  })

  const amountRaw = useMemo(() => {
    if (!isPositiveDecimalAmount(debouncedPayHuman)) return null
    try {
      return tryParseBigInt(toRawAmount(debouncedPayHuman.trim(), UST1_DECIMALS))
    } catch {
      return null
    }
  }, [debouncedPayHuman])

  const nowSec = Math.floor(Date.now() / 1000)
  const balanceRaw = tryParseBigInt(balanceQuery.data ?? '')

  const gate = useMemo(
    () =>
      evaluateUst1SubmitGate({
        windowEnabled,
        walletConnected: !!address,
        direction,
        amountRaw,
        amountDraftEmpty: !payHuman.trim(),
        balanceRaw,
        view: effectiveQuery.data,
        viewLoading: effectiveQuery.isLoading,
        viewError: effectiveQuery.isError,
        submitting: false,
        nowSec,
      }),
    [
      windowEnabled,
      address,
      direction,
      amountRaw,
      payHuman,
      balanceRaw,
      effectiveQuery.data,
      effectiveQuery.isLoading,
      effectiveQuery.isError,
      nowSec,
    ]
  )

  const mutation = useTerraBroadcastMutation({
    mutationFn: async () => {
      if (!address) throw new Error('Connect your wallet')
      const eff = effectiveQuery.data
      if (!eff) throw new Error('Quote unavailable')
      if (amountRaw === null) throw new Error('Enter a valid amount')
      const check = evaluateUst1SubmitGate({
        windowEnabled,
        walletConnected: true,
        direction,
        amountRaw,
        amountDraftEmpty: false,
        balanceRaw,
        view: eff,
        viewLoading: false,
        viewError: false,
        submitting: false,
        nowSec: Math.floor(Date.now() / 1000),
      })
      if (!check.canSubmit) {
        throw new Error(check.statusMessage ?? check.ctaLabel)
      }
      return executeUst1Window(direction, address, amountRaw.toString(), eff)
    },
    onSuccess: (txHash) => {
      sounds.playSuccess()
      setSuccessTx(txHash)
      setPayHuman('')
      void queryClient.invalidateQueries({ queryKey: ['tokenBalance'] })
      void queryClient.invalidateQueries({ queryKey: ['ust1EffectiveSwap'] })
    },
    onError: () => sounds.playError(),
  })

  const canSubmit = gate.canSubmit && !mutation.isPending

  if (!windowEnabled) {
    return (
      <div className="max-w-2xl mx-auto" data-testid="ust1-page">
        <div
          className="shell-panel-strong py-8 text-center"
          style={{ color: 'var(--ink-dim)' }}
          data-testid="ust1-unavailable"
        >
          UST1 window is not configured.
        </div>
      </div>
    )
  }

  const eff = effectiveQuery.data
  const remaining = eff ? rollingRemainingUst1(eff, nowSec) : null
  const paySym = paySymbolForDirection(direction)
  const recvSym = receiveSymbolForDirection(direction)

  return (
    <div className="max-w-2xl mx-auto" data-testid="ust1-page">
      <div className="mb-6">
        <h2 className="text-lg font-semibold mb-1 uppercase tracking-wide font-heading">UST1</h2>
        <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
          Oracle mint and redeem between vFDUSD and UST1. This is not an AMM swap.
        </p>
      </div>

      <div className="shell-panel-strong mb-6">
        <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
          Uses the live ust1-window oracle rate and fee. For market trading of secondary pairs, use{' '}
          <Link to="/" className="underline" style={{ color: 'var(--ink)' }}>
            Swap
          </Link>{' '}
          or{' '}
          <Link to="/trade" className="underline" style={{ color: 'var(--ink)' }}>
            Trade
          </Link>
          .
        </p>
      </div>

      {!address && (
        <div className="shell-panel-strong mb-6 text-center text-sm" style={{ color: 'var(--ink-dim)' }}>
          Connect your wallet to deposit or withdraw.
        </div>
      )}

      {effectiveQuery.isLoading && (
        <div className="shell-panel-strong flex items-center justify-center gap-3 py-8" aria-live="polite">
          <Spinner /> <span style={{ color: 'var(--ink-dim)' }}>Loading window...</span>
        </div>
      )}

      {effectiveQuery.isError && (
        <RetryError
          message={`Failed to load window: ${effectiveQuery.error?.message ?? 'Unknown error'}`}
          onRetry={() => void effectiveQuery.refetch()}
        />
      )}

      {effectiveQuery.isSuccess && eff && (
        <div className="shell-panel-strong space-y-5">
          <div className="flex gap-2" role="tablist" aria-label="UST1 direction" data-testid="ust1-mode-tabs">
            {(
              [
                ['deposit', 'Deposit'],
                ['withdraw', 'Withdraw'],
              ] as const
            ).map(([value, label]) => {
              const active = direction === value
              return (
                <button
                  key={value}
                  type="button"
                  role="tab"
                  aria-selected={active}
                  data-testid={`ust1-tab-${value}`}
                  className={`flex-1 py-2.5 text-sm font-semibold uppercase tracking-wide ${
                    active ? 'btn-primary' : ''
                  }`}
                  style={
                    active
                      ? undefined
                      : {
                          background: 'var(--panel-muted, transparent)',
                          color: 'var(--ink-dim)',
                          border: '1px solid var(--stroke, transparent)',
                        }
                  }
                  onClick={() => {
                    sounds.playButtonPress()
                    setDirection(value)
                    setPayHuman('')
                    setSuccessTx(null)
                  }}
                >
                  {label}
                </button>
              )
            })}
          </div>

          <div className="grid grid-cols-2 gap-3 text-sm" data-testid="ust1-limits">
            <div>
              <p className="label-glass">Fee</p>
              <p className="font-medium tabular-nums">{(eff.fee_bps / 100).toFixed(2)}%</p>
            </div>
            <div>
              <p className="label-glass">Per-tx max</p>
              <p className="font-medium tabular-nums">
                {formatTokenAmountAbbrev(eff.per_tx_ust1_limit, UST1_DECIMALS, 4)} UST1
              </p>
            </div>
            <div>
              <p className="label-glass">24h remaining</p>
              <p className="font-medium tabular-nums">
                {remaining === null ? '—' : `${formatTokenAmountAbbrev(remaining.toString(), UST1_DECIMALS, 4)} UST1`}
              </p>
            </div>
            <div>
              <p className="label-glass">Oracle</p>
              <p className="font-medium" data-testid="ust1-oracle-status">
                {eff.oracle.paused ? 'Paused' : isOracleStale(eff, nowSec) ? 'Stale' : 'Fresh'}
              </p>
            </div>
          </div>

          <div>
            <div className="flex items-center justify-between gap-2 mb-1">
              <label className="label-glass" htmlFor="ust1-pay-amount">
                Pay ({paySym})
              </label>
              {address && balanceQuery.data != null && (
                <button
                  type="button"
                  className="text-xs underline"
                  style={{ color: 'var(--ink-dim)' }}
                  data-testid="ust1-max"
                  onClick={() => {
                    setPayHuman(fromRawAmount(balanceQuery.data!, UST1_DECIMALS))
                  }}
                >
                  Max {formatTokenAmountAbbrev(balanceQuery.data, UST1_DECIMALS, 4)}
                </button>
              )}
            </div>
            <input
              id="ust1-pay-amount"
              data-testid="ust1-pay-amount"
              inputMode="decimal"
              autoComplete="off"
              className="input-glass w-full"
              placeholder="0"
              value={payHuman}
              disabled={!address || mutation.isPending}
              onChange={(e) => {
                const v = e.target.value
                if (isDecimalAmountDraft(v)) setPayHuman(v)
              }}
            />
            <p className="mt-1 text-xs" style={{ color: 'var(--ink-dim)' }}>
              {direction === 'deposit' ? 'Sends vFDUSD only.' : 'Sends UST1 only.'}
            </p>
          </div>

          <div>
            <p className="label-glass">Receive ({recvSym})</p>
            <p
              className="text-2xl font-semibold font-heading tabular-nums"
              style={{ color: 'var(--mint)' }}
              data-testid="ust1-receive-amount"
            >
              {!payHuman.trim()
                ? '—'
                : payHuman !== debouncedPayHuman
                  ? 'Calculating…'
                  : gate.receiveRaw != null
                    ? formatTokenAmountAbbrev(gate.receiveRaw.toString(), UST1_DECIMALS, 6)
                    : '—'}
            </p>
          </div>

          {gate.statusMessage && payHuman.trim() && (
            <p className="text-sm" style={{ color: 'var(--danger, #c44)' }} data-testid="ust1-block-reason">
              {gate.statusMessage}
            </p>
          )}

          <button
            type="button"
            data-testid="ust1-submit"
            onClick={() => {
              sounds.playButtonPress()
              setSuccessTx(null)
              mutation.mutate()
            }}
            disabled={!canSubmit}
            className={`w-full py-3 font-semibold ${canSubmit ? 'btn-primary' : 'btn-disabled !w-full'}`}
          >
            {mutation.isPending ? (direction === 'deposit' ? 'Depositing...' : 'Withdrawing...') : gate.ctaLabel}
          </button>

          <TerraBroadcastPendingLink phase={mutation.phase} txHash={mutation.pendingTxHash} />

          {mutation.isError && (
            <p className="text-sm" style={{ color: 'var(--danger, #c44)' }} data-testid="ust1-error">
              {humanizeUserFacingErrorFromUnknown(mutation.error)}
            </p>
          )}

          {successTx && (
            <p className="text-sm" style={{ color: 'var(--mint)' }} data-testid="ust1-success">
              Submitted. Tx: {successTx.slice(0, 12)}…
            </p>
          )}

          <p className="text-xs text-center" style={{ color: 'var(--ink-dim)' }}>
            You pay network gas for each window transaction.
          </p>
        </div>
      )}
    </div>
  )
}
