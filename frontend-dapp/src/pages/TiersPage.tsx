import React from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { getTiers, getRegistration, register, deregister } from '@/services/terraclassic/feeDiscount'
import { FEE_DISCOUNT_CONTRACT_ADDRESS } from '@/utils/constants'
import type { TierEntry } from '@/types'
import { Spinner, Badge } from '@/components/ui'
import { sounds } from '@/lib/sounds'

const CL8Y_DECIMALS = 18

function formatCl8y(raw: string): string {
  const n = BigInt(raw)
  const divisor = BigInt(10) ** BigInt(CL8Y_DECIMALS)
  const whole = n / divisor
  return whole.toLocaleString()
}

function discountLabel(bps: number): string {
  return `${(bps / 100).toFixed(0)}%`
}

function effectiveFeeLabel(baseBps: number, discountBps: number): string {
  const effective = (baseBps * (10000 - discountBps)) / 10000
  return `${(effective / 100).toFixed(2)}%`
}

function TierRow({
  entry,
  isCurrentTier,
  onRegister,
  isRegistering,
  canSelfRegister,
}: {
  entry: TierEntry
  isCurrentTier: boolean
  onRegister: (tierId: number) => void
  isRegistering: boolean
  canSelfRegister: boolean
}) {
  const { tier_id, tier } = entry

  return (
    <div
      className={`flex items-center gap-4 p-4 rounded-none border-2 transition-colors shadow-[3px_3px_0_#000] ${
        isCurrentTier
          ? 'border-[color:var(--mint)] bg-[color:var(--accent-surface)]'
          : ''
      }`}
      style={isCurrentTier ? undefined : {
        borderColor: 'rgba(255,255,255,0.2)',
        background: 'var(--surface-0)',
      }}
    >
      <div
        className="w-12 h-12 rounded-none border-2 flex items-center justify-center text-lg font-bold shadow-[2px_2px_0_#000]"
        style={{
          borderColor: 'rgba(255,255,255,0.2)',
          background: 'var(--surface-1)',
          color: 'var(--ink)',
          fontFamily: "'Chakra Petch', sans-serif",
        }}
      >
        {tier_id}
      </div>

      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <span className="font-medium uppercase tracking-wide" style={{ color: 'var(--ink)' }}>
            {tier.governance_only
              ? tier_id === 0
                ? 'Market Maker'
                : 'Blacklist'
              : `Tier ${tier_id}`}
          </span>
          {tier.governance_only && (
            <Badge variant="warning">Governance</Badge>
          )}
          {isCurrentTier && (
            <Badge variant="accent">Active</Badge>
          )}
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
        <div className="text-lg font-semibold" style={{ color: 'var(--ink)', fontFamily: "'Chakra Petch', sans-serif" }}>{discountLabel(tier.discount_bps)}</div>
        <div className="text-xs uppercase tracking-wide font-medium" style={{ color: 'var(--ink-subtle)' }}>fee discount</div>
      </div>

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
          <span className="block text-center text-xs uppercase tracking-wide font-medium" style={{ color: 'var(--ink-subtle)' }}>Governance only</span>
        )}
      </div>
    </div>
  )
}

export default function TiersPage() {
  const address = useWalletStore((s) => s.address)
  const queryClient = useQueryClient()

  const tiersQuery = useQuery({
    queryKey: ['feeDiscountTiers'],
    queryFn: () => getTiers(),
    enabled: !!FEE_DISCOUNT_CONTRACT_ADDRESS,
    staleTime: 60_000,
  })

  const registrationQuery = useQuery({
    queryKey: ['feeDiscountRegistration', address],
    queryFn: () => getRegistration(address!),
    enabled: !!address && !!FEE_DISCOUNT_CONTRACT_ADDRESS,
    staleTime: 10_000,
  })

  const registerMutation = useMutation({
    mutationFn: (tierId: number) => register(address!, tierId),
    onSuccess: () => {
      sounds.playSuccess()
      queryClient.invalidateQueries({ queryKey: ['feeDiscountRegistration'] })
    },
    onError: () => sounds.playError(),
  })

  const deregisterMutation = useMutation({
    mutationFn: () => deregister(address!),
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
        <h2 className="text-lg font-semibold mb-1 uppercase tracking-wide" style={{ fontFamily: "'Chakra Petch', sans-serif" }}>Fee Discount Tiers</h2>
        <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
          Hold CL8Y tokens to reduce your swap fees. Register for a tier below.
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
                className="btn-muted !text-xs hover:!border-red-700 hover:!text-red-400 disabled:opacity-50"
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
        <div className="shell-panel-strong flex items-center justify-center gap-3 py-8">
          <Spinner /> <span style={{ color: 'var(--ink-dim)' }}>Loading tiers...</span>
        </div>
      )}

      {tiersQuery.isError && (
        <div className="alert-error py-8 text-center">
          Failed to load tiers: {tiersQuery.error?.message}
        </div>
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
          />
        ))}
      </div>

      {/* Fee Calculation Example */}
      <div className="mt-8 shell-panel-strong">
        <h3 className="text-sm font-semibold uppercase tracking-wide mb-3" style={{ color: 'var(--ink)', fontFamily: "'Chakra Petch', sans-serif" }}>How it works</h3>
        <div className="text-sm space-y-2" style={{ color: 'var(--ink-dim)' }}>
          <p>Your swap fee is reduced based on your registered tier. For a pool with 1.80% base fee:</p>
          <div className="grid grid-cols-3 gap-2 text-xs mt-3">
            <div className="label-neo !mb-0">Tier</div>
            <div className="label-neo !mb-0">Discount</div>
            <div className="label-neo !mb-0">Effective Fee</div>
            {selfRegisterTiers.map((t) => (
              <React.Fragment key={t.tier_id}>
                <div style={{ color: 'var(--ink)' }}>Tier {t.tier_id}</div>
                <div style={{ color: 'var(--cyan)' }}>{discountLabel(t.tier.discount_bps)}</div>
                <div style={{ color: 'var(--ink)' }}>{effectiveFeeLabel(180, t.tier.discount_bps)}</div>
              </React.Fragment>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}
