import React, { memo } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { getTiers, getRegistration, register, deregister } from '@/services/terraclassic/feeDiscount'
import { FEE_DISCOUNT_CONTRACT_ADDRESS, CL8Y_TOKEN_ADDRESS } from '@/utils/constants'
import type { TierEntry } from '@/types'
import { Spinner, Badge, RetryError } from '@/components/ui'
import { sounds } from '@/lib/sounds'
import { formatTokenAmountAbbrev } from '@/utils/formatAmount'
import { lookupByCW20 } from '@/utils/tokenRegistry'
import { getFactoryConfig } from '@/services/terraclassic/settings'

const CL8Y_DECIMALS = lookupByCW20(CL8Y_TOKEN_ADDRESS)?.decimals ?? 18

function formatCl8y(raw: string): string {
  return formatTokenAmountAbbrev(raw, CL8Y_DECIMALS)
}

function discountLabel(bps: number): string {
  const pct = bps / 100
  return pct % 1 === 0 ? `${pct.toFixed(0)}%` : `${pct.toFixed(1)}%`
}

function effectiveFeeLabel(discountBps: number, baseFee = 180): string {
  const effective = (baseFee * (10000 - discountBps)) / 10000
  const pct = effective / 100
  return pct % 1 === 0 ? `${pct.toFixed(1)}%` : `${pct.toFixed(2)}%`
}

const TierRow = memo(function TierRow({
  entry,
  isCurrentTier,
  onRegister,
  isRegistering,
  canSelfRegister,
  baseFee,
}: {
  entry: TierEntry
  isCurrentTier: boolean
  onRegister: (tierId: number) => void
  isRegistering: boolean
  canSelfRegister: boolean
  baseFee: number
}) {
  const { tier_id, tier } = entry

  return (
    <div
      className={`flex items-center gap-4 p-4 rounded-[24px] border transition-colors ${
        isCurrentTier ? 'border-[color:var(--mint)] bg-[color:var(--accent-surface)]' : ''
      }`}
      style={
        isCurrentTier
          ? undefined
          : {
              borderColor: 'rgba(255,255,255,0.2)',
              background: 'var(--surface-0)',
            }
      }
    >
      <div
        className="w-12 h-12 rounded-[18px] border flex items-center justify-center text-lg font-bold font-heading"
        style={{
          borderColor: 'rgba(255,255,255,0.2)',
          background: 'var(--surface-1)',
          color: 'var(--ink)',
        }}
      >
        {tier_id}
      </div>

      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <span className="font-medium uppercase tracking-wide" style={{ color: 'var(--ink)' }}>
            {tier.governance_only ? (tier_id === 0 ? 'Market Maker' : 'Blacklist') : `Tier ${tier_id}`}
          </span>
          {tier.governance_only && <Badge variant="warning">Governance</Badge>}
          {isCurrentTier && <Badge variant="accent">Active</Badge>}
        </div>
        <div className="text-sm" style={{ color: 'var(--ink-dim)' }}>
          {tier.min_cl8y_balance !== '0' ? (
            <span>Hold {formatCl8y(tier.min_cl8y_balance)} CL8Y</span>
          ) : (
            <span>{tier.governance_only ? 'Governance assigned' : 'No holding requirement'}</span>
          )}
        </div>
      </div>

      <div className="text-right">
        <div className="text-lg font-semibold font-heading" style={{ color: 'var(--ink)' }}>
          {discountLabel(tier.discount_bps)}
        </div>
        <div className="text-xs uppercase tracking-wide font-medium" style={{ color: 'var(--ink-subtle)' }}>
          fee discount
        </div>
      </div>

      {!tier.governance_only && (
        <div className="text-right min-w-[4.5rem]">
          <div className="text-lg font-semibold font-heading" style={{ color: 'var(--mint)' }}>
            {effectiveFeeLabel(tier.discount_bps, baseFee)}
          </div>
          <div className="text-xs uppercase tracking-wide font-medium" style={{ color: 'var(--ink-subtle)' }}>
            eff. fee*
          </div>
        </div>
      )}

      <div className="w-28">
        {!tier.governance_only && canSelfRegister && !isCurrentTier && (
          <button
            onClick={() => {
              sounds.playButtonPress()
              onRegister(tier_id)
            }}
            disabled={isRegistering}
            className={`w-full py-2 font-semibold text-sm ${
              isRegistering ? 'btn-disabled !w-full' : 'btn-primary !w-full'
            }`}
          >
            {isRegistering ? '...' : 'Register'}
          </button>
        )}
        {tier.governance_only && (
          <span
            className="block text-center text-xs uppercase tracking-wide font-medium"
            style={{ color: 'var(--ink-subtle)' }}
          >
            Governance only
          </span>
        )}
      </div>
    </div>
  )
})

export default function TiersPage() {
  const address = useWalletStore((s) => s.address)
  const queryClient = useQueryClient()

  const factoryConfigQuery = useQuery({
    queryKey: ['factoryConfig'],
    queryFn: getFactoryConfig,
    staleTime: 120_000,
  })

  const baseFee = factoryConfigQuery.data?.default_fee_bps ?? 180

  const tiersQuery = useQuery({
    queryKey: ['feeDiscountTiers'],
    queryFn: () => getTiers(),
    enabled: !!FEE_DISCOUNT_CONTRACT_ADDRESS,
    staleTime: 60_000,
  })

  const registrationQuery = useQuery({
    queryKey: ['feeDiscountRegistration', address],
    queryFn: () => {
      if (!address) throw new Error('No address')
      return getRegistration(address)
    },
    enabled: !!address && !!FEE_DISCOUNT_CONTRACT_ADDRESS,
    staleTime: 10_000,
  })

  const registerMutation = useMutation({
    mutationFn: (tierId: number) => {
      if (!address) throw new Error('No address')
      return register(address, tierId)
    },
    onSuccess: () => {
      sounds.playSuccess()
      queryClient.invalidateQueries({ queryKey: ['feeDiscountRegistration'] })
    },
    onError: () => sounds.playError(),
  })

  const deregisterMutation = useMutation({
    mutationFn: () => {
      if (!address) throw new Error('No address')
      return deregister(address)
    },
    onSuccess: () => {
      sounds.playSuccess()
      queryClient.invalidateQueries({ queryKey: ['feeDiscountRegistration'] })
    },
    onError: () => sounds.playError(),
  })

  const tiers = tiersQuery.data ?? []
  const registration = registrationQuery.data
  const currentTierId = registration?.tier_id ?? null
  const isOnGovernanceTier = registration?.tier?.governance_only === true
  const canSelfRegister = !!address && !isOnGovernanceTier

  const selfRegisterTiers = tiers.filter((t) => !t.tier.governance_only)

  if (!FEE_DISCOUNT_CONTRACT_ADDRESS) {
    return (
      <div className="max-w-2xl mx-auto">
        <div className="shell-panel-strong py-8 text-center" style={{ color: 'var(--ink-dim)' }}>
          Fee discount contract not configured.
        </div>
      </div>
    )
  }

  return (
    <div className="max-w-2xl mx-auto">
      <div className="mb-6">
        <h2 className="text-lg font-semibold mb-1 uppercase tracking-wide font-heading">Fee Discount Tiers</h2>
        <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
          Hold CL8Y tokens to reduce swap fees, then register for the tier that matches your balance.
        </p>
      </div>

      {/* Current Status */}
      {address && (
        <div className="shell-panel-strong mb-6">
          <div className="flex items-center justify-between">
            <div>
              <p className="label-neo">Your Status</p>
              {registration?.registered ? (
                <div>
                  <span className="font-medium" style={{ color: 'var(--ink)' }}>
                    {registration.tier?.governance_only
                      ? currentTierId === 0
                        ? 'Market Maker'
                        : currentTierId === 255
                          ? 'Restricted'
                          : `Tier ${currentTierId}`
                      : `Tier ${currentTierId}`}
                  </span>
                  <span className="ml-2" style={{ color: 'var(--cyan)' }}>
                    {discountLabel(registration.tier?.discount_bps ?? 0)} discount
                  </span>
                </div>
              ) : (
                <span style={{ color: 'var(--ink-subtle)' }}>Not registered</span>
              )}
            </div>
            {registration?.registered && !isOnGovernanceTier && (
              <button
                onClick={() => {
                  sounds.playButtonPress()
                  deregisterMutation.mutate()
                }}
                disabled={deregisterMutation.isPending}
                className="btn-muted !text-xs disabled:opacity-50"
              >
                {deregisterMutation.isPending ? 'Deregistering...' : 'Deregister'}
              </button>
            )}
          </div>
          {(registerMutation.isError || deregisterMutation.isError) && (
            <div className="mt-3 alert-error !text-xs">
              {registerMutation.error?.message || deregisterMutation.error?.message}
            </div>
          )}
        </div>
      )}

      {!address && (
        <div className="shell-panel-strong mb-6 text-center text-sm" style={{ color: 'var(--ink-dim)' }}>
          Connect your wallet to register for a fee discount tier.
        </div>
      )}

      {/* Tier List */}
      {tiersQuery.isLoading && (
        <div className="shell-panel-strong flex items-center justify-center gap-3 py-8" aria-live="polite">
          <Spinner /> <span style={{ color: 'var(--ink-dim)' }}>Loading tiers...</span>
        </div>
      )}

      {tiersQuery.isError && (
        <RetryError
          message={`Failed to load tiers: ${tiersQuery.error?.message ?? 'Unknown error'}`}
          onRetry={() => void tiersQuery.refetch()}
        />
      )}

      <div className="space-y-3">
        {selfRegisterTiers.map((entry) => (
          <TierRow
            key={entry.tier_id}
            entry={entry}
            isCurrentTier={currentTierId === entry.tier_id}
            onRegister={(tierId) => registerMutation.mutate(tierId)}
            isRegistering={registerMutation.isPending}
            canSelfRegister={canSelfRegister}
            baseFee={baseFee}
          />
        ))}
      </div>

      {/* How it works */}
      <div className="mt-8 shell-panel-strong">
        <h3 className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading" style={{ color: 'var(--ink)' }}>
          How it works
        </h3>
        <div className="text-sm space-y-2" style={{ color: 'var(--ink-dim)' }}>
          <p>
            Your swap fee is reduced based on your registered tier. If you drop below the required CL8Y holding at any
            time, you lose your tier.
          </p>
          <p className="text-xs" style={{ color: 'var(--ink-subtle)' }}>
            The default base fee is {(baseFee / 100).toFixed(1)}% for most pairs. Some pairs may have a different base
            fee &mdash; your tier discount applies as a percentage off whichever base fee the pair uses.
          </p>
          <div className="grid grid-cols-4 gap-2 text-xs mt-3">
            <div className="label-neo !mb-0">Tier</div>
            <div className="label-neo !mb-0">CL8Y Hold</div>
            <div className="label-neo !mb-0">Discount</div>
            <div className="label-neo !mb-0">Eff. Fee*</div>
            <div style={{ color: 'var(--ink-subtle)' }}>No tier</div>
            <div style={{ color: 'var(--ink-subtle)' }}>&mdash;</div>
            <div style={{ color: 'var(--ink-subtle)' }}>&mdash;</div>
            <div style={{ color: 'var(--ink-subtle)' }}>{(baseFee / 100).toFixed(1)}%</div>
            {selfRegisterTiers.map((t) => (
              <React.Fragment key={t.tier_id}>
                <div style={{ color: 'var(--ink)' }}>Tier {t.tier_id}</div>
                <div style={{ color: 'var(--ink)' }}>{formatCl8y(t.tier.min_cl8y_balance)}</div>
                <div style={{ color: 'var(--cyan)' }}>{discountLabel(t.tier.discount_bps)}</div>
                <div style={{ color: 'var(--mint)' }}>{effectiveFeeLabel(t.tier.discount_bps, baseFee)}</div>
              </React.Fragment>
            ))}
          </div>
          <p className="text-xs mt-2" style={{ color: 'var(--ink-subtle)' }}>
            *Effective fee shown assumes the default {(baseFee / 100).toFixed(1)}% base fee. Pairs with a custom base
            fee will have a proportionally different effective fee.
          </p>
        </div>
      </div>
    </div>
  )
}
