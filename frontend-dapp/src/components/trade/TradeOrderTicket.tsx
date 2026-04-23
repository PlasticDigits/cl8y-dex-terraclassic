import { useMemo, useState, useEffect } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { placeLimitOrder, cancelLimitOrder, getPairPaused } from '@/services/terraclassic/pair'
import { executeTerraContract } from '@/services/terraclassic/transactions'
import { getPairLimitPlacements } from '@/services/indexer/client'
import { sounds } from '@/lib/sounds'
import { TxResultAlert, Spinner } from '@/components/ui'
import { assetInfoLabel, tokenAssetInfo, type PairInfo } from '@/types'
import { getDecimals, toRawAmount } from '@/utils/formatAmount'
import { fetchCW20TokenInfo, getTokenDisplaySymbol } from '@/utils/tokenDisplay'
import { DOCS_GITLAB_BASE } from '@/utils/constants'
import { useLimitOrderForm } from '@/hooks/useLimitOrderForm'
import { useLimitOrderEscrowBalance } from '@/hooks/useLimitOrderEscrowBalance'
import { LimitOrderEscrowAmountField } from '@/components/trade/LimitOrderEscrowAmountField'
import { LimitOrderExpiryField } from '@/components/trade/LimitOrderExpiryField'
import { LimitOrderAdvancedLimitSettings } from '@/components/trade/LimitOrderAdvancedLimitSettings'

/**
 * Limit place / cancel for the trade workspace (pair is chosen by parent).
 */
export function TradeOrderTicket({
  pairAddr,
  pairs,
  pairsLoading,
}: {
  pairAddr: string
  pairs: PairInfo[]
  pairsLoading: boolean
}) {
  const address = useWalletStore((s) => s.address)
  const openWalletModal = useWalletStore((s) => s.openWalletModal)
  const wallet = getConnectedWallet()
  const isWalletConnected = !!address && !!wallet
  const queryClient = useQueryClient()

  const [side, setSide] = useState<'bid' | 'ask'>('bid')
  const [price, setPrice] = useState('1')
  const {
    maxSteps,
    setMaxSteps,
    expiresAt,
    setExpiresAt,
    amountHuman,
    setAmountHuman,
    limitAdvancedOpen,
    setLimitAdvancedOpen,
  } = useLimitOrderForm()
  const [cancelOrderId, setCancelOrderId] = useState('')
  const [lastIndexedOrderId, setLastIndexedOrderId] = useState<number | null>(null)

  const selectedPair = useMemo(() => pairs.find((p) => p.contract_addr === pairAddr), [pairs, pairAddr])

  const token0 = selectedPair ? assetInfoLabel(selectedPair.asset_infos[0]) : ''
  const token1 = selectedPair ? assetInfoLabel(selectedPair.asset_infos[1]) : ''
  const escrowToken = side === 'bid' ? token1 : token0
  const escrowDecimals = escrowToken ? getDecimals(tokenAssetInfo(escrowToken)) : 6
  const escrowBalanceQuery = useLimitOrderEscrowBalance(address, escrowToken)

  const placementsQuery = useQuery({
    queryKey: ['limitPlacements', pairAddr],
    queryFn: () => getPairLimitPlacements(pairAddr, { limit: 100 }),
    enabled: pairAddr.startsWith('terra1'),
  })

  const pausedQuery = useQuery({
    queryKey: ['pairPaused', pairAddr],
    queryFn: () => getPairPaused(pairAddr),
    enabled: pairAddr.startsWith('terra1'),
    staleTime: 15_000,
  })

  const isPaused = pausedQuery.data?.paused === true

  const myPlacements = useMemo(() => {
    if (!address || !placementsQuery.data) return []
    return placementsQuery.data.filter((r) => r.owner === address)
  }, [address, placementsQuery.data])

  const placeMutation = useMutation({
    mutationFn: async () => {
      if (!address) throw new Error('Connect wallet')
      if (!selectedPair) throw new Error('Select a pair')
      if (!escrowToken.startsWith('terra1')) throw new Error('Escrow token must be CW20')
      const raw = toRawAmount(amountHuman, escrowDecimals)
      if (raw === '0') throw new Error('Enter amount')
      await executeTerraContract(address, escrowToken, {
        increase_allowance: { spender: selectedPair.contract_addr, amount: raw },
      })
      return placeLimitOrder(address, escrowToken, selectedPair.contract_addr, raw, side, price, maxSteps, expiresAt)
    },
    onSuccess: async () => {
      sounds.playSuccess()
      setAmountHuman('')
      queryClient.invalidateQueries({ queryKey: ['limitPlacements'] })
      queryClient.invalidateQueries({ queryKey: ['tokenBalance'] })
      queryClient.invalidateQueries({ queryKey: ['limitBookPage', pairAddr] })
      setLastIndexedOrderId(null)
      const addr = pairAddr
      const walletAddr = address
      if (!addr.startsWith('terra1') || !walletAddr) return
      for (let i = 0; i < 24; i++) {
        await new Promise((r) => setTimeout(r, 500))
        try {
          const rows = await getPairLimitPlacements(addr, { limit: 100 })
          const mine = rows.filter((r) => r.owner === walletAddr)
          const maxId = mine.reduce((m, r) => Math.max(m, r.order_id), 0)
          if (maxId > 0) {
            setLastIndexedOrderId(maxId)
            setCancelOrderId(String(maxId))
            break
          }
        } catch {
          /* indexer optional in local dev */
        }
      }
    },
    onError: () => sounds.playError(),
  })

  const cancelMutation = useMutation({
    mutationFn: async () => {
      if (!address) throw new Error('Connect wallet')
      if (!pairAddr.startsWith('terra1')) throw new Error('Select a pair')
      const id = parseInt(cancelOrderId, 10)
      if (!Number.isFinite(id) || id < 1) throw new Error('Invalid order id')
      return cancelLimitOrder(address, pairAddr, id)
    },
    onSuccess: () => {
      sounds.playSuccess()
      setCancelOrderId('')
      setLastIndexedOrderId(null)
      queryClient.invalidateQueries({ queryKey: ['limitCancellations'] })
      queryClient.invalidateQueries({ queryKey: ['limitBookPage', pairAddr] })
    },
    onError: () => sounds.playError(),
  })

  useEffect(() => {
    pairs.forEach((p) => {
      const a = assetInfoLabel(p.asset_infos[0])
      const b = assetInfoLabel(p.asset_infos[1])
      if (a.startsWith('terra1')) void fetchCW20TokenInfo(a)
      if (b.startsWith('terra1')) void fetchCW20TokenInfo(b)
    })
  }, [pairs])

  useEffect(() => {
    setLastIndexedOrderId(null)
  }, [pairAddr])

  if (pairsLoading) {
    return (
      <div className="flex justify-center py-12 card-neo">
        <Spinner />
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-3 h-full min-h-0 overflow-y-auto card-neo !p-4">
      <div>
        <h3 className="text-xs font-semibold uppercase tracking-wide">Order ticket</h3>
        {!pairAddr.startsWith('terra1') && (
          <p className="text-[10px] mt-1" style={{ color: 'var(--ink-dim)' }}>
            Select a trading pair from the bar above.
          </p>
        )}
      </div>

      {selectedPair && (
        <div className="text-[10px] uppercase tracking-wide font-medium" style={{ color: 'var(--ink-dim)' }}>
          Token0: {token0.slice(0, 14)}… · Token1: {token1.slice(0, 14)}…
        </div>
      )}

      {selectedPair && isPaused && (
        <div className="alert-error text-xs space-y-2" role="status">
          <p>Pair is paused — limit place/cancel blocked until governance unpauses.</p>
          <a
            className="underline text-[10px]"
            href={`${DOCS_GITLAB_BASE}/contracts-security-audit.md`}
            target="_blank"
            rel="noopener noreferrer"
          >
            L6 (audit)
          </a>
        </div>
      )}

      <div className="space-y-3 border-t border-white/10 pt-3">
        <h3 className="text-xs font-semibold uppercase tracking-wide">Place limit</h3>
        <div className="flex flex-wrap gap-3 text-xs">
          <label className="flex items-center gap-2 cursor-pointer">
            <input type="radio" name="trade-side" checked={side === 'bid'} onChange={() => setSide('bid')} />
            Bid ({getTokenDisplaySymbol(token1 || 'token1')})
          </label>
          <label className="flex items-center gap-2 cursor-pointer">
            <input type="radio" name="trade-side" checked={side === 'ask'} onChange={() => setSide('ask')} />
            Ask ({getTokenDisplaySymbol(token0 || 'token0')})
          </label>
        </div>
        <div>
          <label className="label-neo">Price (token1 per token0)</label>
          <input
            className="input-neo w-full font-mono text-sm"
            value={price}
            onChange={(e) => setPrice(e.target.value)}
          />
        </div>
        <LimitOrderEscrowAmountField
          compact
          escrowLabel={getTokenDisplaySymbol(escrowToken || '—')}
          escrowDecimals={escrowDecimals}
          amountHuman={amountHuman}
          onAmountChange={setAmountHuman}
          balanceQuery={escrowBalanceQuery}
          onMax={setAmountHuman}
          walletConnected={isWalletConnected}
        />
        <LimitOrderExpiryField compact value={expiresAt} onChange={setExpiresAt} idPrefix="trade-ticket" />
        <LimitOrderAdvancedLimitSettings
          compact
          open={limitAdvancedOpen}
          onOpenChange={setLimitAdvancedOpen}
          maxSteps={maxSteps}
          onMaxStepsChange={setMaxSteps}
          expiresAt={expiresAt}
          onExpiresAtChange={setExpiresAt}
          idPrefix="trade-ticket"
        />
        <button
          type="button"
          className="btn-primary btn-cta w-full !text-xs"
          disabled={!isWalletConnected || placeMutation.isPending || !selectedPair || isPaused}
          onClick={() => {
            if (!isWalletConnected) openWalletModal()
            else placeMutation.mutate()
          }}
        >
          {!isWalletConnected ? 'Connect Wallet' : placeMutation.isPending ? 'Placing…' : 'Place limit'}
        </button>
        {placeMutation.isError && <TxResultAlert type="error" message={(placeMutation.error as Error).message} />}
        {placeMutation.isSuccess && (
          <TxResultAlert type="success" message="Limit order submitted." txHash={placeMutation.data} />
        )}
        {lastIndexedOrderId != null && (
          <p className="text-[10px] font-mono" data-testid="trade-last-placed-order-id">
            Last indexed: #{lastIndexedOrderId}
          </p>
        )}
      </div>

      <div className="space-y-3 border-t border-white/10 pt-3">
        <h3 className="text-xs font-semibold uppercase tracking-wide">Cancel limit</h3>
        <input
          className="input-neo w-full font-mono text-sm"
          value={cancelOrderId}
          onChange={(e) => setCancelOrderId(e.target.value)}
          placeholder="Order ID"
        />
        <button
          type="button"
          className="btn-primary btn-cta w-full !text-xs"
          disabled={!isWalletConnected || cancelMutation.isPending || !pairAddr || isPaused}
          onClick={() => {
            if (!isWalletConnected) openWalletModal()
            else cancelMutation.mutate()
          }}
        >
          {!isWalletConnected ? 'Connect Wallet' : cancelMutation.isPending ? 'Cancelling…' : 'Cancel'}
        </button>
        {cancelMutation.isError && <TxResultAlert type="error" message={(cancelMutation.error as Error).message} />}
        {cancelMutation.isSuccess && (
          <TxResultAlert type="success" message="Cancel submitted." txHash={cancelMutation.data} />
        )}
      </div>

      {pairAddr && address && (
        <div className="text-[10px] font-mono space-y-1 border-t border-white/10 pt-3 max-h-28 overflow-y-auto">
          <div className="uppercase tracking-wide font-semibold mb-1" style={{ color: 'var(--ink-dim)' }}>
            Your placements
          </div>
          {placementsQuery.isLoading && <Spinner />}
          {!placementsQuery.isLoading &&
            myPlacements.map((r) => (
              <div key={r.id}>
                #{r.order_id} · {r.side ?? '?'} · {r.price ?? '?'}
              </div>
            ))}
        </div>
      )}
    </div>
  )
}
