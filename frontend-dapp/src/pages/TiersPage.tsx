import React from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { getTiers, getRegistration, register, deregister } from '@/services/terraclassic/feeDiscount'
import { FEE_DISCOUNT_CONTRACT_ADDRESS } from '@/utils/constants'
import type { TierEntry } from '@/types'

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
      className={`flex items-center gap-4 p-4 rounded-xl border transition-colors ${
        isCurrentTier
          ? 'border-dex-accent bg-dex-accent/5'
          : 'border-dex-border bg-dex-bg'
      }`}
    >
      <div className="w-12 h-12 rounded-xl bg-dex-card border border-dex-border flex items-center justify-center text-lg font-bold text-white">
        {tier_id}
      </div>

      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <span className="font-medium text-white">
            {tier.governance_only
              ? tier_id === 0
                ? 'Market Maker'
                : 'Blacklist'
              : `Tier ${tier_id}`}
          </span>
          {tier.governance_only && (
            <span className="text-[10px] px-1.5 py-0.5 rounded bg-amber-500/15 text-amber-400 border border-amber-500/20">
              Governance
            </span>
          )}
          {isCurrentTier && (
            <span className="text-[10px] px-1.5 py-0.5 rounded bg-dex-accent/15 text-dex-accent border border-dex-accent/20">
              Active
            </span>
          )}
        </div>
        <div className="text-sm text-gray-400">
          {tier.min_cl8y_balance !== '0' ? (
            <span>Hold {formatCl8y(tier.min_cl8y_balance)} CL8Y</span>
          ) : (
            <span>{tier.governance_only ? 'Governance assigned' : 'No holding requirement'}</span>
          )}
        </div>
      </div>

      <div className="text-right">
        <div className="text-lg font-semibold text-white">{discountLabel(tier.discount_bps)}</div>
        <div className="text-xs text-gray-500">fee discount</div>
      </div>

      <div className="w-28">
        {!tier.governance_only && canSelfRegister && !isCurrentTier && (
          <button
            onClick={() => onRegister(tier_id)}
            disabled={isRegistering}
            className="w-full py-2 rounded-lg text-sm font-medium transition-colors bg-dex-accent text-dex-bg hover:bg-dex-accent/80 disabled:bg-dex-border disabled:text-gray-500 disabled:cursor-not-allowed"
          >
            {isRegistering ? '...' : 'Register'}
          </button>
        )}
        {tier.governance_only && (
          <span className="block text-center text-xs text-gray-500">Governance only</span>
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
      queryClient.invalidateQueries({ queryKey: ['feeDiscountRegistration'] })
    },
  })

  const deregisterMutation = useMutation({
    mutationFn: () => deregister(address!),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['feeDiscountRegistration'] })
    },
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
        <div className="bg-dex-card rounded-2xl border border-dex-border p-8 text-center text-gray-400">
          Fee discount contract not configured.
        </div>
      </div>
    )
  }

  return (
    <div className="max-w-2xl mx-auto">
      <div className="mb-6">
        <h2 className="text-lg font-semibold mb-1">Fee Discount Tiers</h2>
        <p className="text-sm text-gray-400">
          Hold CL8Y tokens to reduce your swap fees. Register for a tier below.
        </p>
      </div>

      {/* Current Status */}
      {address && (
        <div className="bg-dex-card rounded-2xl border border-dex-border p-5 mb-6">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-xs text-gray-400 mb-1">Your Status</p>
              {registration?.registered ? (
                <div>
                  <span className="text-white font-medium">
                    {registration.tier?.governance_only
                      ? currentTierId === 0
                        ? 'Market Maker'
                        : currentTierId === 255
                          ? 'Restricted'
                          : `Tier ${currentTierId}`
                      : `Tier ${currentTierId}`}
                  </span>
                  <span className="text-dex-accent ml-2">
                    {discountLabel(registration.tier?.discount_bps ?? 0)} discount
                  </span>
                </div>
              ) : (
                <span className="text-gray-500">Not registered</span>
              )}
            </div>
            {registration?.registered && !isOnGovernanceTier && (
              <button
                onClick={() => deregisterMutation.mutate()}
                disabled={deregisterMutation.isPending}
                className="px-4 py-2 rounded-lg text-sm border border-dex-border text-gray-300 hover:border-red-500/50 hover:text-red-400 transition-colors disabled:opacity-50"
              >
                {deregisterMutation.isPending ? 'Deregistering...' : 'Deregister'}
              </button>
            )}
          </div>
          {(registerMutation.isError || deregisterMutation.isError) && (
            <p className="mt-3 text-sm text-red-400">
              {registerMutation.error?.message || deregisterMutation.error?.message}
            </p>
          )}
        </div>
      )}

      {!address && (
        <div className="bg-dex-card rounded-2xl border border-dex-border p-5 mb-6 text-center text-gray-400 text-sm">
          Connect your wallet to register for a fee discount tier.
        </div>
      )}

      {/* Tier List */}
      {tiersQuery.isLoading && (
        <div className="bg-dex-card rounded-2xl border border-dex-border p-8 text-center text-gray-400 animate-pulse">
          Loading tiers...
        </div>
      )}

      {tiersQuery.isError && (
        <div className="bg-dex-card rounded-2xl border border-dex-border p-8 text-center text-red-400">
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
      <div className="mt-8 bg-dex-card rounded-2xl border border-dex-border p-5">
        <h3 className="text-sm font-medium text-white mb-3">How it works</h3>
        <div className="text-sm text-gray-400 space-y-2">
          <p>Your swap fee is reduced based on your registered tier. For a pool with 1.80% base fee:</p>
          <div className="grid grid-cols-3 gap-2 text-xs mt-3">
            <div className="text-gray-500 font-medium">Tier</div>
            <div className="text-gray-500 font-medium">Discount</div>
            <div className="text-gray-500 font-medium">Effective Fee</div>
            {selfRegisterTiers.map((t) => (
              <React.Fragment key={t.tier_id}>
                <div className="text-gray-300">Tier {t.tier_id}</div>
                <div className="text-dex-accent">{discountLabel(t.tier.discount_bps)}</div>
                <div className="text-white">{effectiveFeeLabel(180, t.tier.discount_bps)}</div>
              </React.Fragment>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}
