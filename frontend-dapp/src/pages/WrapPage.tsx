import { useMemo, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { useDebouncedValue } from '@/hooks/useDebouncedValue'
import { useTerraBroadcastMutation } from '@/hooks/useTerraBroadcastMutation'
import { useTokenBalance } from '@/hooks/useTokenBalance'
import { isNativeWrapEnabled, LUNC_C_TOKEN_ADDRESS, USTC_C_TOKEN_ADDRESS } from '@/utils/constants'
import { executeNativeSwap, simulateNativeSwap } from '@/services/terraclassic/router'
import {
  checkRateLimitExceeded,
  queryPausedState,
  queryWrapMapperConfig,
  wrapMapperFeeBps,
  wrapTreasuryMatchesEnv,
  wrapUnwrapFeeNote,
} from '@/services/terraclassic/wrapMapper'
import { formatTokenAmountAbbrev, fromRawAmount, toRawAmount } from '@/utils/formatAmount'
import { isDecimalAmountDraft, isPositiveDecimalAmount, tryParseBigInt } from '@/utils/decimalAmountInput'
import { computeMaxSpendableHumanAmount } from '@/utils/maxSpendableAmount'
import { lookupByCW20, lookupByDenom } from '@/utils/tokenRegistry'
import { Spinner, RetryError, TokenLogo, TxResultAlert } from '@/components/ui'
import { TerraBroadcastPendingLink } from '@/components/ui/TerraBroadcastPendingLink'
import { sounds } from '@/lib/sounds'
import { humanizeUserFacingErrorFromUnknown } from '@/utils/humanizeUserFacingError'
import {
  WRAP_CONFIG_UNAVAILABLE_CTA,
  WRAP_RATE_LIMIT_EXCEEDED_MESSAGE,
  WRAP_TREASURY_MISCONFIGURED_CTA,
  WRAP_UNWRAP_EXCHANGE_DEPOSIT_WARNING,
} from '@/utils/marketDataServiceCopy'
import { fetchNativeTransferTaxParams } from '@/utils/nativeTransferTax'
import { WrapRateLimitStatus } from '@/components/wrap/WrapRateLimitStatus'

const DECIMALS = 6
const QUOTE_DEBOUNCE_MS = 250

type WrapMode = 'wrap' | 'unwrap'
type WrapAsset = 'lunc' | 'ustc'

function nativeDenom(asset: WrapAsset): 'uluna' | 'uusd' {
  return asset === 'lunc' ? 'uluna' : 'uusd'
}

function wrappedAddr(asset: WrapAsset): string {
  return asset === 'lunc' ? LUNC_C_TOKEN_ADDRESS : USTC_C_TOKEN_ADDRESS
}

function payToken(mode: WrapMode, asset: WrapAsset): string {
  return mode === 'wrap' ? nativeDenom(asset) : wrappedAddr(asset)
}

function receiveToken(mode: WrapMode, asset: WrapAsset): string {
  return mode === 'wrap' ? wrappedAddr(asset) : nativeDenom(asset)
}

function symbolFor(tokenId: string): string {
  if (tokenId === 'uluna') return 'LUNC'
  if (tokenId === 'uusd') return 'USTC'
  if (tokenId === LUNC_C_TOKEN_ADDRESS) return 'cLUNC'
  if (tokenId === USTC_C_TOKEN_ADDRESS) return 'cUSTC'
  return tokenId.slice(0, 8)
}

function tokenLogoProps(tokenId: string) {
  const sym = symbolFor(tokenId)
  if (tokenId === 'uluna' || tokenId === 'uusd') {
    const entry = lookupByDenom(tokenId)
    return {
      addressForBlockie: undefined as string | undefined,
      blockieSeed: tokenId,
      logoURI: entry?.logoURI,
      symbol: sym,
    }
  }
  const entry = lookupByCW20(tokenId)
  return {
    addressForBlockie: tokenId || undefined,
    blockieSeed: tokenId ? undefined : sym,
    logoURI: entry?.logoURI,
    symbol: sym,
  }
}

export default function WrapPage() {
  const address = useWalletStore((s) => s.address)
  const queryClient = useQueryClient()
  const [mode, setMode] = useState<WrapMode>('wrap')
  const [asset, setAsset] = useState<WrapAsset>('lunc')
  const [payHuman, setPayHuman] = useState('')
  const [successTx, setSuccessTx] = useState<string | null>(null)
  const debouncedPayHuman = useDebouncedValue(payHuman, QUOTE_DEBOUNCE_MS)

  const wrapEnabled = isNativeWrapEnabled()
  const fromToken = payToken(mode, asset)
  const toToken = receiveToken(mode, asset)
  const balanceQuery = useTokenBalance(address, fromToken)

  const configQuery = useQuery({
    queryKey: ['wrapMapperConfig'],
    queryFn: queryWrapMapperConfig,
    enabled: wrapEnabled,
    staleTime: 30_000,
    refetchInterval: 30_000, // W14 — do not quote a stale unwrap fee after set_fees
  })

  const pausedQuery = useQuery({
    queryKey: ['wrapMapperPaused'],
    queryFn: queryPausedState,
    enabled: wrapEnabled,
    staleTime: 30_000,
  })

  const amountRaw = useMemo(() => {
    if (!isPositiveDecimalAmount(debouncedPayHuman)) return null
    try {
      return tryParseBigInt(toRawAmount(debouncedPayHuman.trim(), DECIMALS))
    } catch {
      return null
    }
  }, [debouncedPayHuman])

  const rateLimitQuery = useQuery({
    queryKey: ['rateLimit', nativeDenom(asset), amountRaw?.toString() ?? '0'],
    queryFn: () => checkRateLimitExceeded(nativeDenom(asset), amountRaw!.toString()),
    enabled: wrapEnabled && mode === 'wrap' && amountRaw != null && amountRaw > 0n,
    staleTime: 15_000,
  })

  const unwrapBurnTaxQuery = useQuery({
    queryKey: ['nativeTransferTax', nativeDenom(asset)],
    queryFn: () => fetchNativeTransferTaxParams(nativeDenom(asset)),
    enabled: wrapEnabled && mode === 'unwrap',
    staleTime: 60_000,
  })

  const quoteQuery = useQuery({
    queryKey: ['wrapPageQuote', mode, fromToken, toToken, amountRaw?.toString() ?? ''],
    queryFn: () => simulateNativeSwap(amountRaw!.toString(), fromToken, toToken, []),
    enabled: wrapEnabled && amountRaw != null && amountRaw > 0n,
    staleTime: 15_000,
  })

  const config = configQuery.data ?? null
  const feeBps = wrapMapperFeeBps(config, mode)
  const treasuryMismatch = !!config && !wrapTreasuryMatchesEnv(config)
  const configUnavailable = wrapEnabled && configQuery.isFetched && config == null
  const pauseUnknown = wrapEnabled && pausedQuery.isFetched && pausedQuery.data === null
  const rateLimitUnknown =
    mode === 'wrap' && amountRaw != null && amountRaw > 0n && rateLimitQuery.isFetched && rateLimitQuery.data === null
  const safetyUnavailable = configUnavailable || pauseUnknown || rateLimitUnknown
  const isPaused = pausedQuery.data === true || config?.paused === true
  const rateLimited = rateLimitQuery.data === true

  const balanceRaw = tryParseBigInt(balanceQuery.data ?? '')
  const payIsNativeUluna = fromToken === 'uluna'
  const maxResult = useMemo(() => {
    if (!balanceQuery.data) {
      return { human: '0', spendableRaw: 0n, cappedByGas: false, reserveUluna: 0n }
    }
    return computeMaxSpendableHumanAmount({
      balanceRaw: balanceQuery.data,
      decimals: DECIMALS,
      assetIsNativeUluna: payIsNativeUluna,
      context: payIsNativeUluna ? 'swap_native' : 'swap_cw20',
      nativeSwapHints: payIsNativeUluna
        ? { isDirectWrap: mode === 'wrap', needsWrapInput: false, needsUnwrapOutput: false }
        : undefined,
    })
  }, [balanceQuery.data, payIsNativeUluna, mode])

  const insufficientBalance = amountRaw != null && balanceRaw != null && amountRaw > balanceRaw

  let ctaLabel = mode === 'wrap' ? 'Wrap' : 'Unwrap'
  let canSubmit = false
  let statusMessage: string | null = null

  if (!wrapEnabled) {
    ctaLabel = 'Unavailable'
  } else if (!address) {
    ctaLabel = 'Connect Wallet'
  } else if (treasuryMismatch) {
    ctaLabel = WRAP_TREASURY_MISCONFIGURED_CTA
  } else if (safetyUnavailable) {
    ctaLabel = WRAP_CONFIG_UNAVAILABLE_CTA
  } else if (isPaused) {
    ctaLabel = 'Wrapping is Temporarily Paused'
  } else if (!payHuman.trim()) {
    ctaLabel = 'Enter Amount'
  } else if (amountRaw == null) {
    ctaLabel = 'Enter Amount'
  } else if (rateLimited) {
    ctaLabel = 'Rate Limit Exceeded'
    statusMessage = WRAP_RATE_LIMIT_EXCEEDED_MESSAGE
  } else if (insufficientBalance) {
    ctaLabel = 'Insufficient Balance'
  } else if (quoteQuery.isError) {
    ctaLabel = 'Quote unavailable'
  } else if (quoteQuery.isLoading || payHuman !== debouncedPayHuman) {
    ctaLabel = 'Calculating…'
  } else if (quoteQuery.data) {
    ctaLabel = mode === 'wrap' ? 'Wrap' : 'Unwrap'
    canSubmit = true
  } else {
    ctaLabel = 'Quote unavailable'
  }

  const mutation = useTerraBroadcastMutation({
    mutationFn: async () => {
      if (!address) throw new Error('Connect your wallet')
      if (amountRaw == null) throw new Error('Enter a valid amount')
      if (treasuryMismatch) throw new Error(WRAP_TREASURY_MISCONFIGURED_CTA)
      if (safetyUnavailable) throw new Error(WRAP_CONFIG_UNAVAILABLE_CTA)
      if (isPaused) throw new Error('Wrapping is Temporarily Paused')
      if (rateLimited) throw new Error(WRAP_RATE_LIMIT_EXCEEDED_MESSAGE)
      return executeNativeSwap(address, fromToken, toToken, amountRaw.toString(), [], '0.05')
    },
    onSuccess: (txHash) => {
      sounds.playSuccess()
      setSuccessTx(txHash)
      setPayHuman('')
      void queryClient.invalidateQueries({ queryKey: ['tokenBalance'] })
      void queryClient.invalidateQueries({ queryKey: ['wrapMapperConfig'] })
      void queryClient.invalidateQueries({ queryKey: ['rateLimit'] })
    },
    onError: () => sounds.playError(),
  })

  const submitEnabled = canSubmit && !mutation.isPending

  if (!wrapEnabled) {
    return (
      <div className="max-w-2xl mx-auto" data-testid="wrap-page">
        <div
          className="shell-panel-strong py-8 text-center"
          style={{ color: 'var(--ink-dim)' }}
          data-testid="wrap-unavailable"
        >
          Native wrap is not configured.
        </div>
      </div>
    )
  }

  const payLogo = tokenLogoProps(fromToken)
  const recvLogo = tokenLogoProps(toToken)
  const receiveRaw = quoteQuery.data?.amount

  return (
    <div className="max-w-2xl mx-auto" data-testid="wrap-page">
      <div className="mb-6">
        <h2 className="text-lg font-semibold mb-1 uppercase tracking-wide font-heading">Wrap</h2>
      </div>

      {!address && (
        <div className="shell-panel-strong mb-6 text-center text-sm" style={{ color: 'var(--ink-dim)' }}>
          Connect your wallet to wrap or unwrap.
        </div>
      )}

      {configQuery.isLoading && (
        <div className="shell-panel-strong flex items-center justify-center gap-3 py-8" aria-live="polite">
          <Spinner /> <span style={{ color: 'var(--ink-dim)' }}>Loading wrap config...</span>
        </div>
      )}

      {configQuery.isError && (
        <RetryError
          message={`Failed to load wrap config: ${configQuery.error?.message ?? 'Unknown error'}`}
          onRetry={() => void configQuery.refetch()}
        />
      )}

      {(configQuery.isSuccess || configUnavailable) && (
        <div className="shell-panel-strong space-y-5">
          <div className="flex gap-2" role="tablist" aria-label="Wrap direction" data-testid="wrap-mode-tabs">
            {(
              [
                ['wrap', 'Wrap'],
                ['unwrap', 'Unwrap'],
              ] as const
            ).map(([value, label]) => {
              const active = mode === value
              return (
                <button
                  key={value}
                  type="button"
                  role="tab"
                  aria-selected={active}
                  data-testid={`wrap-tab-${value}`}
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
                    setMode(value)
                    setPayHuman('')
                    setSuccessTx(null)
                  }}
                >
                  {label}
                </button>
              )
            })}
          </div>

          <div className="flex gap-2" role="group" aria-label="Wrap asset" data-testid="wrap-asset-tabs">
            {(
              [
                ['lunc', 'LUNC', 'cLUNC', 'uluna', LUNC_C_TOKEN_ADDRESS] as const,
                ['ustc', 'USTC', 'cUSTC', 'uusd', USTC_C_TOKEN_ADDRESS] as const,
              ] as const
            ).map(([value, nativeSym, wrappedSym, nativeId, wrappedId]) => {
              const active = asset === value
              const nativeLogo = tokenLogoProps(nativeId)
              const wrappedLogo = tokenLogoProps(wrappedId)
              return (
                <button
                  key={value}
                  type="button"
                  data-testid={`wrap-asset-${value}`}
                  className={`flex-1 py-2 px-2 text-xs font-semibold uppercase tracking-wide inline-flex items-center justify-center gap-2 ${
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
                    setAsset(value)
                    setPayHuman('')
                    setSuccessTx(null)
                  }}
                >
                  <span className="inline-flex items-center -space-x-1.5" aria-hidden>
                    <TokenLogo
                      addressForBlockie={nativeLogo.addressForBlockie}
                      blockieSeed={nativeLogo.blockieSeed}
                      logoURI={nativeLogo.logoURI}
                      size={22}
                    />
                    <TokenLogo
                      addressForBlockie={wrappedLogo.addressForBlockie}
                      blockieSeed={wrappedLogo.blockieSeed}
                      logoURI={wrappedLogo.logoURI}
                      size={22}
                    />
                  </span>
                  <span>
                    {nativeSym} / {wrappedSym}
                  </span>
                </button>
              )
            })}
          </div>

          <div className="text-sm" data-testid="wrap-fee-panel">
            <p className="label-glass">Fee</p>
            <p className="font-medium tabular-nums" data-testid="wrap-fee-note">
              {wrapUnwrapFeeNote(mode, feeBps, mode === 'unwrap' ? unwrapBurnTaxQuery.data?.rate : null)}
            </p>
          </div>

          {mode === 'unwrap' && (
            <p
              className="text-xs leading-snug"
              style={{ color: 'var(--color-warning, #f59e0b)' }}
              data-testid="wrap-unwrap-exchange-warning"
              role="note"
            >
              {WRAP_UNWRAP_EXCHANGE_DEPOSIT_WARNING}
            </p>
          )}

          <WrapRateLimitStatus
            denom={nativeDenom(asset)}
            symbol={asset === 'lunc' ? 'LUNC' : 'USTC'}
            enabled={wrapEnabled}
            testId="wrap-page-rate-limit"
          />

          <div>
            <div className="flex items-center justify-between gap-2 mb-1">
              <label className="label-glass inline-flex items-center gap-1.5" htmlFor="wrap-pay-amount">
                <TokenLogo
                  addressForBlockie={payLogo.addressForBlockie}
                  blockieSeed={payLogo.blockieSeed}
                  logoURI={payLogo.logoURI}
                  size={18}
                />
                <span data-testid="wrap-pay-symbol">Pay ({payLogo.symbol})</span>
              </label>
              {address && balanceQuery.data != null && (
                <button
                  type="button"
                  className="text-xs underline"
                  style={{ color: 'var(--ink-dim)' }}
                  data-testid="wrap-max"
                  onClick={() => {
                    setPayHuman(maxResult.human || fromRawAmount(balanceQuery.data!, DECIMALS))
                  }}
                >
                  Max {formatTokenAmountAbbrev(maxResult.spendableRaw.toString(), DECIMALS, 4)}
                </button>
              )}
            </div>
            <input
              id="wrap-pay-amount"
              data-testid="wrap-pay-amount"
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
          </div>

          <div>
            <p className="label-glass inline-flex items-center gap-1.5">
              <TokenLogo
                addressForBlockie={recvLogo.addressForBlockie}
                blockieSeed={recvLogo.blockieSeed}
                logoURI={recvLogo.logoURI}
                size={18}
              />
              <span data-testid="wrap-receive-symbol">Receive ({recvLogo.symbol})</span>
            </p>
            <p
              className="text-2xl font-semibold font-heading tabular-nums"
              style={{ color: 'var(--mint)' }}
              data-testid="wrap-receive-amount"
            >
              {!payHuman.trim()
                ? '—'
                : payHuman !== debouncedPayHuman || quoteQuery.isLoading
                  ? 'Calculating…'
                  : receiveRaw
                    ? formatTokenAmountAbbrev(receiveRaw, DECIMALS, 6)
                    : '—'}
            </p>
          </div>

          {statusMessage && payHuman.trim() && (
            <p className="text-sm" style={{ color: 'var(--danger, #c44)' }} data-testid="wrap-block-reason">
              {statusMessage}
            </p>
          )}

          <button
            type="button"
            data-testid="wrap-submit"
            onClick={() => {
              sounds.playButtonPress()
              setSuccessTx(null)
              mutation.mutate()
            }}
            disabled={!submitEnabled}
            className={`w-full py-3 font-semibold ${submitEnabled ? 'btn-primary' : 'btn-disabled !w-full'}`}
          >
            {mutation.isPending ? (mode === 'wrap' ? 'Wrapping...' : 'Unwrapping...') : ctaLabel}
          </button>

          <TerraBroadcastPendingLink phase={mutation.phase} txHash={mutation.pendingTxHash} />

          {mutation.isError && (
            <p className="text-sm" style={{ color: 'var(--danger, #c44)' }} data-testid="wrap-error">
              {humanizeUserFacingErrorFromUnknown(mutation.error)}
            </p>
          )}

          {successTx && (
            <div data-testid="wrap-success">
              <TxResultAlert type="success" message="Submitted." txHash={successTx} />
            </div>
          )}
        </div>
      )}
    </div>
  )
}
