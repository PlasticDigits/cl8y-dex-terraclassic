import { useEffect, useMemo, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { isFaucetEnabled, SOFT_LAUNCH_MINTABLE_TOKENS } from '@/utils/constants'
import { drip, getFaucetConfig, getFaucetCooldown } from '@/services/terraclassic/faucet'
import { formatFaucetCooldown } from '@/utils/faucetCooldown'
import { formatTokenAmountAbbrev } from '@/utils/formatAmount'
import { Spinner, RetryError, TokenSelect } from '@/components/ui'
import { sounds } from '@/lib/sounds'
import { humanizeUserFacingErrorFromUnknown } from '@/utils/humanizeUserFacingError'

export default function MintPage() {
  const address = useWalletStore((s) => s.address)
  const queryClient = useQueryClient()
  const [selectedToken, setSelectedToken] = useState('')
  const [successTx, setSuccessTx] = useState<string | null>(null)

  const mintableAddresses = useMemo(() => SOFT_LAUNCH_MINTABLE_TOKENS.map((t) => t.address), [])

  useEffect(() => {
    if (!selectedToken && mintableAddresses.length > 0) {
      setSelectedToken(mintableAddresses[0])
    }
  }, [mintableAddresses, selectedToken])

  const configQuery = useQuery({
    queryKey: ['faucetConfig'],
    queryFn: getFaucetConfig,
    enabled: isFaucetEnabled(),
    staleTime: 60_000,
  })

  const cooldownQuery = useQuery({
    queryKey: ['faucetCooldown', address],
    queryFn: () => {
      if (!address) throw new Error('No address')
      return getFaucetCooldown(address)
    },
    enabled: !!address && isFaucetEnabled(),
    refetchInterval: (query) => {
      const remaining = query.state.data?.seconds_remaining ?? 0
      return remaining > 0 ? 1_000 : false
    },
    staleTime: 0,
  })

  const dripMutation = useMutation({
    mutationFn: () => {
      if (!address) throw new Error('Connect your wallet')
      if (!selectedToken) throw new Error('Select a token')
      return drip(address, selectedToken)
    },
    onSuccess: (txHash) => {
      sounds.playSuccess()
      setSuccessTx(txHash)
      queryClient.invalidateQueries({ queryKey: ['tokenBalance'] })
      queryClient.invalidateQueries({ queryKey: ['faucetCooldown'] })
    },
    onError: () => sounds.playError(),
  })

  const config = configQuery.data
  const cooldown = cooldownQuery.data
  const dripAmountLabel = config ? formatTokenAmountAbbrev(config.drip_amount, 6, 4) : '100'
  const onCooldown = !!cooldown && !cooldown.can_claim && cooldown.seconds_remaining > 0
  const faucetPaused = config?.paused === true || cooldown?.paused === true
  const canMint =
    !!address &&
    !!selectedToken &&
    !onCooldown &&
    !faucetPaused &&
    !dripMutation.isPending &&
    mintableAddresses.length > 0

  if (!isFaucetEnabled()) {
    return (
      <div className="max-w-2xl mx-auto" data-testid="mint-page">
        <div
          className="shell-panel-strong py-8 text-center"
          style={{ color: 'var(--ink-dim)' }}
          data-testid="mint-unavailable"
        >
          Mint faucet is not available.
        </div>
      </div>
    )
  }

  return (
    <div className="max-w-2xl mx-auto" data-testid="mint-page">
      <div className="mb-6">
        <h2 className="text-lg font-semibold mb-1 uppercase tracking-wide font-heading">Mint</h2>
        <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
          Claim demo tokens for soft-launch testing. These tokens have no economic value.
        </p>
      </div>

      <div className="shell-panel-strong mb-6">
        <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
          <strong style={{ color: 'var(--ink)' }}>Demo only.</strong> Minted tokens are for protocol testing and
          demonstration. They are not investments, securities, or a promise of future value. Do not rely on them for
          financial decisions.
        </p>
      </div>

      {!address && (
        <div className="shell-panel-strong mb-6 text-center text-sm" style={{ color: 'var(--ink-dim)' }}>
          Connect your wallet to mint demo tokens.
        </div>
      )}

      {configQuery.isLoading && (
        <div className="shell-panel-strong flex items-center justify-center gap-3 py-8" aria-live="polite">
          <Spinner /> <span style={{ color: 'var(--ink-dim)' }}>Loading faucet...</span>
        </div>
      )}

      {configQuery.isError && (
        <RetryError
          message={`Failed to load faucet: ${configQuery.error?.message ?? 'Unknown error'}`}
          onRetry={() => void configQuery.refetch()}
        />
      )}

      {configQuery.isSuccess && (
        <div className="shell-panel-strong space-y-5">
          {mintableAddresses.length === 0 ? (
            <p className="text-sm text-center" style={{ color: 'var(--ink-dim)' }} data-testid="mint-unavailable">
              No mintable tokens configured.
            </p>
          ) : (
            <>
              <div>
                <p className="label-glass">Token</p>
                <div data-testid="mint-token-select">
                  <TokenSelect
                    value={selectedToken}
                    tokens={mintableAddresses}
                    onChange={setSelectedToken}
                    aria-label="Mint token"
                    disabled={!address || dripMutation.isPending}
                  />
                </div>
              </div>

              <div>
                <p className="label-glass">Amount per mint</p>
                <p className="text-2xl font-semibold font-heading" style={{ color: 'var(--mint)' }}>
                  {dripAmountLabel}
                </p>
              </div>

              {address && cooldownQuery.isLoading && (
                <div className="flex items-center gap-2 text-sm" style={{ color: 'var(--ink-dim)' }}>
                  <Spinner size="sm" /> Checking cooldown...
                </div>
              )}

              {address && cooldownQuery.isError && (
                <RetryError
                  message={`Failed to load cooldown: ${cooldownQuery.error?.message ?? 'Unknown error'}`}
                  onRetry={() => void cooldownQuery.refetch()}
                />
              )}

              {address && cooldownQuery.isSuccess && (
                <div data-testid="mint-cooldown">
                  {faucetPaused ? (
                    <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
                      Faucet is paused.
                    </p>
                  ) : onCooldown ? (
                    <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
                      Next mint available in{' '}
                      <span className="font-medium tabular-nums" style={{ color: 'var(--ink)' }}>
                        {formatFaucetCooldown(cooldown.seconds_remaining)}
                      </span>
                    </p>
                  ) : (
                    <p className="text-sm" style={{ color: 'var(--mint)' }}>
                      Ready to mint
                    </p>
                  )}
                </div>
              )}

              <button
                type="button"
                data-testid="mint-submit"
                onClick={() => {
                  sounds.playButtonPress()
                  setSuccessTx(null)
                  dripMutation.mutate()
                }}
                disabled={!canMint}
                className={`w-full py-3 font-semibold ${canMint ? 'btn-primary' : 'btn-disabled !w-full'}`}
              >
                {dripMutation.isPending ? 'Minting...' : 'Mint'}
              </button>

              <p className="text-xs text-center" style={{ color: 'var(--ink-subtle)' }}>
                You pay network gas for each mint transaction.
              </p>

              {dripMutation.isError && (
                <div className="alert-error !text-xs">{humanizeUserFacingErrorFromUnknown(dripMutation.error)}</div>
              )}

              {successTx && (
                <div className="alert-success !text-xs">Mint successful. Balances will update shortly.</div>
              )}
            </>
          )}
        </div>
      )}
    </div>
  )
}
