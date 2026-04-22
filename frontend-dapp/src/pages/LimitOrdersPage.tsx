import { useMemo, useState, useEffect } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import { placeLimitOrder, cancelLimitOrder, getPairPaused } from '@/services/terraclassic/pair'
import { executeTerraContract } from '@/services/terraclassic/transactions'
import { getPairLimitBookPage, getPairLimitPlacements } from '@/services/indexer/client'
import { sounds } from '@/lib/sounds'
import { MenuSelect, TxResultAlert, Spinner } from '@/components/ui'
import { assetInfoLabel, tokenAssetInfo } from '@/types'
import { getDecimals, toRawAmount } from '@/utils/formatAmount'
import { pairInfosToMenuSelectOptions } from '@/utils/pairMenuOptions'
import { fetchCW20TokenInfo, getTokenDisplaySymbol, shortenAddress } from '@/utils/tokenDisplay'
import { DOCS_GITLAB_BASE } from '@/utils/constants'

export default function LimitOrdersPage() {
  const address = useWalletStore((s) => s.address)
  const openWalletModal = useWalletStore((s) => s.openWalletModal)
  const wallet = getConnectedWallet()
  const isWalletConnected = !!address && !!wallet
  const queryClient = useQueryClient()

  const [pairAddr, setPairAddr] = useState('')
  const [side, setSide] = useState<'bid' | 'ask'>('bid')
  const [price, setPrice] = useState('1')
  const [amountHuman, setAmountHuman] = useState('')
  const [maxSteps, setMaxSteps] = useState(32)
  const [expiresUnix, setExpiresUnix] = useState('')
  const [cancelOrderId, setCancelOrderId] = useState('')
  const [lastIndexedOrderId, setLastIndexedOrderId] = useState<number | null>(null)

  const pairsQuery = useQuery({
    queryKey: ['allPairs'],
    queryFn: () => getAllPairsPaginated(),
    staleTime: 60_000,
  })

  const pairs = pairsQuery.data?.pairs ?? []

  const pairMenuOptions = useMemo(() => pairInfosToMenuSelectOptions(pairs, { variant: 'full' }), [pairs])

  const selectedPair = useMemo(() => pairs.find((p) => p.contract_addr === pairAddr), [pairs, pairAddr])

  const token0 = selectedPair ? assetInfoLabel(selectedPair.asset_infos[0]) : ''
  const token1 = selectedPair ? assetInfoLabel(selectedPair.asset_infos[1]) : ''
  const escrowToken = side === 'bid' ? token1 : token0

  const escrowDecimals = escrowToken ? getDecimals(tokenAssetInfo(escrowToken)) : 6

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

  const bookBidQuery = useQuery({
    queryKey: ['limitBookPagePreview', pairAddr, 'bid'],
    queryFn: () => getPairLimitBookPage(pairAddr, 'bid', { limit: 20 }),
    enabled: pairAddr.startsWith('terra1'),
    staleTime: 10_000,
  })

  const bookAskQuery = useQuery({
    queryKey: ['limitBookPagePreview', pairAddr, 'ask'],
    queryFn: () => getPairLimitBookPage(pairAddr, 'ask', { limit: 20 }),
    enabled: pairAddr.startsWith('terra1'),
    staleTime: 10_000,
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
      return placeLimitOrder(
        address,
        escrowToken,
        selectedPair.contract_addr,
        raw,
        side,
        price,
        maxSteps,
        expiresUnix.trim() ? Number(expiresUnix.trim()) : null
      )
    },
    onSuccess: async () => {
      sounds.playSuccess()
      setAmountHuman('')
      queryClient.invalidateQueries({ queryKey: ['limitPlacements'] })
      queryClient.invalidateQueries({ queryKey: ['tokenBalance'] })
      queryClient.invalidateQueries({ queryKey: ['limitBookPagePreview', pairAddr] })
      setLastIndexedOrderId(null)
      const addr = pairAddr
      const wallet = address
      if (!addr.startsWith('terra1') || !wallet) return
      for (let i = 0; i < 24; i++) {
        await new Promise((r) => setTimeout(r, 500))
        try {
          const rows = await getPairLimitPlacements(addr, { limit: 100 })
          const mine = rows.filter((r) => r.owner === wallet)
          const maxId = mine.reduce((m, r) => Math.max(m, r.order_id), 0)
          if (maxId > 0) {
            setLastIndexedOrderId(maxId)
            setCancelOrderId(String(maxId))
            break
          }
        } catch {
          /* indexer may be down in local dev */
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
      queryClient.invalidateQueries({ queryKey: ['limitBookPagePreview', pairAddr] })
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

  return (
    <div className="max-w-[560px] mx-auto">
      <div className="relative">
        <div
          aria-hidden="true"
          className="pointer-events-none absolute inset-x-6 top-8 h-[78%] rounded-[28px] theme-hero-glow blur-2xl"
        />
        <div className="shell-panel-strong relative z-10">
          <div className="mb-6">
            <h1 className="text-lg font-semibold uppercase tracking-wide font-heading">Limit Orders</h1>
            <p className="text-sm mt-1" style={{ color: 'var(--ink-dim)' }}>
              Place or cancel on-chain limits on a pair. Bids escrow token1; asks escrow token0 (pair ordering).
            </p>
          </div>

          {pairsQuery.isLoading && (
            <div className="flex items-center gap-2 py-8 justify-center">
              <Spinner />
            </div>
          )}

          {!pairsQuery.isLoading && (
            <div className="space-y-6">
              <div>
                <label className="label-neo" htmlFor="limit-pair">
                  Pair
                </label>
                <MenuSelect
                  id="limit-pair"
                  className="relative w-full"
                  aria-label="Trading pair"
                  value={pairAddr}
                  options={pairMenuOptions}
                  emptyLabel="No pairs on factory"
                  onChange={setPairAddr}
                />
              </div>

              {selectedPair && (
                <div className="text-xs uppercase tracking-wide font-medium" style={{ color: 'var(--ink-dim)' }}>
                  Token0: {shortenAddress(token0)} · Token1: {shortenAddress(token1)}
                </div>
              )}

              {selectedPair && isPaused && (
                <div className="alert-error text-sm space-y-2" role="status">
                  <p>
                    This pair is paused by governance. New limit orders and cancel are unavailable until the pair is
                    unpaused (invariant L6 — resting escrow stays locked).
                  </p>
                  <p className="text-xs opacity-90">
                    <a
                      className="underline hover:opacity-80"
                      href={`${DOCS_GITLAB_BASE}/contracts-security-audit.md`}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      Security audit (L6)
                    </a>
                    {' · '}
                    <a
                      className="underline hover:opacity-80"
                      href={`${DOCS_GITLAB_BASE}/limit-orders.md`}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      limit-orders.md
                    </a>
                  </p>
                </div>
              )}

              {selectedPair && (
                <div className="card-neo !p-4 space-y-3">
                  <h2 className="text-sm font-semibold uppercase tracking-wide">On-chain order book (indexer → LCD)</h2>
                  {(bookBidQuery.isLoading || bookAskQuery.isLoading) && <Spinner />}
                  {(bookBidQuery.isError || bookAskQuery.isError) && (
                    <p className="text-xs" style={{ color: 'var(--ink-dim)' }}>
                      Book unavailable (indexer or LCD).
                    </p>
                  )}
                  {!bookBidQuery.isLoading && !bookBidQuery.isError && bookBidQuery.data && (
                    <div>
                      <div
                        className="text-xs font-medium uppercase tracking-wide mb-1"
                        style={{ color: 'var(--ink-dim)' }}
                      >
                        Bids
                      </div>
                      <ul className="text-xs font-mono space-y-1 max-h-32 overflow-y-auto">
                        {bookBidQuery.data.orders.length === 0 && <li className="opacity-70">(empty)</li>}
                        {bookBidQuery.data.orders.map((o) => (
                          <li key={`bid-${o.order_id}`}>
                            #{o.order_id} · {o.price} · rem {o.remaining}
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}
                  {!bookAskQuery.isLoading && !bookAskQuery.isError && bookAskQuery.data && (
                    <div>
                      <div
                        className="text-xs font-medium uppercase tracking-wide mb-1"
                        style={{ color: 'var(--ink-dim)' }}
                      >
                        Asks
                      </div>
                      <ul className="text-xs font-mono space-y-1 max-h-32 overflow-y-auto">
                        {bookAskQuery.data.orders.length === 0 && <li className="opacity-70">(empty)</li>}
                        {bookAskQuery.data.orders.map((o) => (
                          <li key={`ask-${o.order_id}`}>
                            #{o.order_id} · {o.price} · rem {o.remaining}
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}
                </div>
              )}

              <div className="card-neo !p-4 space-y-4">
                <h2 className="text-sm font-semibold uppercase tracking-wide">Place limit</h2>
                <div className="flex gap-4">
                  <label className="flex items-center gap-2 text-sm cursor-pointer">
                    <input type="radio" name="side" checked={side === 'bid'} onChange={() => setSide('bid')} />
                    Bid (escrow {getTokenDisplaySymbol(token1 || 'token1')})
                  </label>
                  <label className="flex items-center gap-2 text-sm cursor-pointer">
                    <input type="radio" name="side" checked={side === 'ask'} onChange={() => setSide('ask')} />
                    Ask (escrow {getTokenDisplaySymbol(token0 || 'token0')})
                  </label>
                </div>
                <div>
                  <label className="label-neo">Price (token1 per token0)</label>
                  <input
                    className="input-neo w-full font-mono"
                    value={price}
                    onChange={(e) => setPrice(e.target.value)}
                  />
                </div>
                <div>
                  <label className="label-neo">Amount ({getTokenDisplaySymbol(escrowToken || '—')})</label>
                  <input
                    className="input-neo w-full"
                    value={amountHuman}
                    onChange={(e) => setAmountHuman(e.target.value)}
                    placeholder="0.0"
                  />
                </div>
                <div>
                  <label className="label-neo">Max adjust steps</label>
                  <input
                    type="number"
                    className="input-neo w-full"
                    min={1}
                    max={256}
                    value={maxSteps}
                    onChange={(e) => setMaxSteps(Number(e.target.value) || 32)}
                  />
                </div>
                <div>
                  <label className="label-neo">Expires at (Unix seconds, optional)</label>
                  <input
                    className="input-neo w-full font-mono"
                    value={expiresUnix}
                    onChange={(e) => setExpiresUnix(e.target.value)}
                    placeholder="Leave empty for no expiry"
                  />
                </div>
                <button
                  type="button"
                  className="btn-primary btn-cta w-full"
                  disabled={!isWalletConnected || placeMutation.isPending || !selectedPair || isPaused}
                  onClick={() => {
                    if (!isWalletConnected) openWalletModal()
                    else placeMutation.mutate()
                  }}
                >
                  {!isWalletConnected ? 'Connect Wallet' : placeMutation.isPending ? 'Placing…' : 'Place limit'}
                </button>
                {placeMutation.isError && (
                  <TxResultAlert type="error" message={(placeMutation.error as Error).message} />
                )}
                {placeMutation.isSuccess && (
                  <TxResultAlert type="success" message="Limit order submitted." txHash={placeMutation.data} />
                )}
                {lastIndexedOrderId != null && (
                  <p className="text-xs font-mono" data-testid="last-placed-order-id">
                    Last indexed placement for your wallet: order #{lastIndexedOrderId}
                  </p>
                )}
              </div>

              <div className="card-neo !p-4 space-y-4">
                <h2 className="text-sm font-semibold uppercase tracking-wide">Cancel limit</h2>
                <div>
                  <label className="label-neo">Order ID</label>
                  <input
                    className="input-neo w-full font-mono"
                    value={cancelOrderId}
                    onChange={(e) => setCancelOrderId(e.target.value)}
                    placeholder="e.g. 42"
                  />
                </div>
                <button
                  type="button"
                  className="btn-primary btn-cta w-full"
                  disabled={!isWalletConnected || cancelMutation.isPending || !pairAddr || isPaused}
                  onClick={() => {
                    if (!isWalletConnected) openWalletModal()
                    else cancelMutation.mutate()
                  }}
                >
                  {!isWalletConnected ? 'Connect Wallet' : cancelMutation.isPending ? 'Cancelling…' : 'Cancel limit'}
                </button>
                {cancelMutation.isError && (
                  <TxResultAlert type="error" message={(cancelMutation.error as Error).message} />
                )}
                {cancelMutation.isSuccess && (
                  <TxResultAlert type="success" message="Cancel transaction submitted." txHash={cancelMutation.data} />
                )}
              </div>

              {pairAddr && address && (
                <div className="card-neo !p-4 space-y-2">
                  <h2 className="text-sm font-semibold uppercase tracking-wide">Your recent placements (indexer)</h2>
                  {placementsQuery.isLoading && <Spinner />}
                  {!placementsQuery.isLoading && myPlacements.length === 0 && (
                    <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
                      No indexed placements for this wallet on this pair (or pair code predates owner attrs).
                    </p>
                  )}
                  <ul className="text-xs font-mono space-y-1 max-h-40 overflow-y-auto">
                    {myPlacements.map((r) => (
                      <li key={r.id}>
                        order #{r.order_id} · {r.side ?? '?'} · {r.price ?? '?'} · {r.block_timestamp.slice(0, 19)}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
