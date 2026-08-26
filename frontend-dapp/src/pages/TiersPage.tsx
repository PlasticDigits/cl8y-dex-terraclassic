import React, { memo } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { getTiers, getRegistration, register, deregister } from '@/services/terraclassic/feeDiscount'
import { FEE_DISCOUNT_CONTRACT_ADDRESS, CL8Y_TOKEN_ADDRESS } from '@/utils/constants'
import type { Tier, TierEntry } from '@/types'
import { Spinner, Badge, RetryError } from '@/components/ui'
import { sounds } from '@/lib/sounds'
import { formatTokenAmountAbbrev } from '@/utils/formatAmount'
import { lookupByCW20 } from '@/utils/tokenRegistry'
import { getFactoryConfig } from '@/services/terraclassic/settings'
import { humanizeUserFacingErrorFromUnknown } from '@/utils/humanizeUserFacingError'
import {
  effectiveSwapFeeBps,
  makerPlacementFeeBps,
  resolveLimitDiscountBps,
  bpsToPercentLabel,
} from '@/utils/limitOrderFeeSummary'

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

function limitPlaceLabel(tier: { discount_bps: number; limit_discount_bps?: number | null }, baseFee = 180): string {
  const limitDisc = resolveLimitDiscountBps(tier.discount_bps, tier.limit_discount_bps)
  return bpsToPercentLabel(makerPlacementFeeBps(effectiveSwapFeeBps(baseFee, limitDisc)))
}

function holdPhrase(tier: Tier): string {
  if (tier.min_cl8y_balance !== '0') {
    return `Hold ${formatCl8y(tier.min_cl8y_balance)} CL8Y`
  }
  return tier.governance_only ? 'Governance assigned' : 'No holding requirement'
}

function tierTitle(tierId: number, governanceOnly: boolean): string {
  if (!governanceOnly) return `Tier ${tierId}`
  if (tierId === 0) return 'Market Maker'
  if (tierId === 255) return 'Restricted'
  return 'Blacklist'
}

const TierFeeStat = memo(function TierFeeStat({
  value,
  label,
  valueColor,
}: {
  value: string
  label: string
  valueColor: string
}) {
  return (
    <div className="text-right shrink-0">
      <div className="text-lg font-semibold font-heading whitespace-nowrap" style={{ color: valueColor }}>
        {value}
      </div>
      <div
        className="text-xs uppercase tracking-wide font-medium whitespace-nowrap"
        style={{ color: 'var(--ink-subtle)' }}
      >
        {label}
      </div>
    </div>
  )
})

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
  const showRegister = !tier.governance_only && canSelfRegister && !isCurrentTier
  const hold = holdPhrase(tier)

  return (
    <div
      data-testid={`tier-card-${tier_id}`}
      className={`flex flex-col gap-3 p-4 rounded-[24px] border transition-colors md:flex-row md:items-center md:gap-4 ${
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
      <div className="flex flex-wrap items-center gap-3 min-w-0 flex-1">
        <div className="flex items-center gap-3 min-w-0">
          <div
            className="w-12 h-12 shrink-0 rounded-[18px] border flex items-center justify-center text-lg font-bold font-heading"
            style={{
              borderColor: 'rgba(255,255,255,0.2)',
              background: 'var(--surface-1)',
              color: 'var(--ink)',
            }}
          >
            {tier_id}
          </div>

          <div className="min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <span className="font-medium tracking-wide whitespace-nowrap" style={{ color: 'var(--ink)' }}>
                {tierTitle(tier_id, tier.governance_only)}
              </span>
              {tier.governance_only && <Badge variant="warning">Governance</Badge>}
              {isCurrentTier && <Badge variant="accent">Active</Badge>}
            </div>
            <div className="text-sm" style={{ color: 'var(--ink-dim)' }}>
              <span data-testid={`tier-hold-${tier_id}`} className="whitespace-nowrap">
                {hold}
              </span>
            </div>
          </div>
        </div>

        <div className="flex shrink-0 items-start gap-3 ml-auto" data-testid={`tier-fee-cluster-${tier_id}`}>
          <TierFeeStat value={discountLabel(tier.discount_bps)} label="fee discount" valueColor="var(--ink)" />
          {!tier.governance_only && (
            <TierFeeStat
              value={effectiveFeeLabel(tier.discount_bps, baseFee)}
              label="eff. fee*"
              valueColor="var(--mint)"
            />
          )}
        </div>
      </div>

      {showRegister && (
        <div className="w-full md:w-auto md:shrink-0">
          <button
            type="button"
            data-testid={`register-tier-${tier_id}`}
            onClick={() => {
              sounds.playButtonPress()
              onRegister(tier_id)
            }}
            disabled={isRegistering}
            className={`w-full min-h-11 md:w-28 py-2 font-semibold text-sm ${
              isRegistering ? 'btn-disabled !w-full md:!w-28' : 'btn-primary !w-full md:!w-28'
            }`}
          >
            {isRegistering ? '...' : 'Register'}
          </button>
        </div>
      )}
    </div>
  )
})

function HowItWorksLadder({ selfRegisterTiers, baseFee }: { selfRegisterTiers: TierEntry[]; baseFee: number }) {
  const noTierLimit = bpsToPercentLabel(Math.floor(baseFee / 2))
  const noTierEff = `${(baseFee / 100).toFixed(1)}%`

  return (
    <>
      <div className="hidden md:grid grid-cols-5 gap-2 text-xs mt-3" data-testid="tiers-how-it-works-table">
        <div className="label-glass !mb-0">Tier</div>
        <div className="label-glass !mb-0">CL8Y Hold</div>
        <div className="label-glass !mb-0">Discount</div>
        <div className="label-glass !mb-0">Eff. Fee*</div>
        <div className="label-glass !mb-0">Limit place*</div>
        <div style={{ color: 'var(--ink-subtle)' }}>No tier</div>
        <div style={{ color: 'var(--ink-subtle)' }}>&mdash;</div>
        <div style={{ color: 'var(--ink-subtle)' }}>&mdash;</div>
        <div style={{ color: 'var(--ink-subtle)' }}>{noTierEff}</div>
        <div style={{ color: 'var(--ink-subtle)' }}>{noTierLimit}</div>
        {selfRegisterTiers.map((t) => (
          <React.Fragment key={t.tier_id}>
            <div style={{ color: 'var(--ink)' }}>Tier {t.tier_id}</div>
            <div style={{ color: 'var(--ink)' }}>{formatCl8y(t.tier.min_cl8y_balance)}</div>
            <div style={{ color: 'var(--cyan)' }}>{discountLabel(t.tier.discount_bps)}</div>
            <div style={{ color: 'var(--mint)' }}>{effectiveFeeLabel(t.tier.discount_bps, baseFee)}</div>
            <div style={{ color: 'var(--cyan)' }}>{limitPlaceLabel(t.tier, baseFee)}</div>
          </React.Fragment>
        ))}
      </div>

      <div className="md:hidden mt-3 space-y-3" data-testid="tiers-how-it-works-mobile">
        <HowItWorksMobileRow title="No tier" hold="—" discount="—" effFee={noTierEff} limitPlace={noTierLimit} muted />
        {selfRegisterTiers.map((t) => (
          <HowItWorksMobileRow
            key={t.tier_id}
            title={`Tier ${t.tier_id}`}
            hold={formatCl8y(t.tier.min_cl8y_balance)}
            discount={discountLabel(t.tier.discount_bps)}
            effFee={effectiveFeeLabel(t.tier.discount_bps, baseFee)}
            limitPlace={limitPlaceLabel(t.tier, baseFee)}
          />
        ))}
      </div>
    </>
  )
}

function HowItWorksMobileRow({
  title,
  hold,
  discount,
  effFee,
  limitPlace,
  muted,
}: {
  title: string
  hold: string
  discount: string
  effFee: string
  limitPlace: string
  muted?: boolean
}) {
  const ink = muted ? 'var(--ink-subtle)' : 'var(--ink)'
  return (
    <div
      className="rounded-[16px] border px-3 py-2"
      style={{ borderColor: 'rgba(255,255,255,0.12)', background: 'var(--surface-1)' }}
      data-testid={`how-it-works-row-${title.replace(/\s+/g, '-').toLowerCase()}`}
    >
      <div className="font-medium whitespace-nowrap mb-1" style={{ color: ink }}>
        {title}
      </div>
      <dl className="grid grid-cols-2 gap-x-3 gap-y-1 text-xs">
        <dt className="whitespace-nowrap" style={{ color: 'var(--ink-subtle)' }}>
          CL8Y Hold
        </dt>
        <dd className="text-right whitespace-nowrap" style={{ color: ink }}>
          {hold}
        </dd>
        <dt className="whitespace-nowrap" style={{ color: 'var(--ink-subtle)' }}>
          Discount
        </dt>
        <dd className="text-right whitespace-nowrap" style={{ color: muted ? 'var(--ink-subtle)' : 'var(--cyan)' }}>
          {discount}
        </dd>
        <dt className="whitespace-nowrap" style={{ color: 'var(--ink-subtle)' }}>
          Eff. Fee*
        </dt>
        <dd className="text-right whitespace-nowrap" style={{ color: muted ? 'var(--ink-subtle)' : 'var(--mint)' }}>
          {effFee}
        </dd>
        <dt className="whitespace-nowrap" style={{ color: 'var(--ink-subtle)' }}>
          Limit place*
        </dt>
        <dd className="text-right whitespace-nowrap" style={{ color: muted ? 'var(--ink-subtle)' : 'var(--cyan)' }}>
          {limitPlace}
        </dd>
      </dl>
    </div>
  )
}

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
        <div
          className="shell-panel-strong py-8 text-center"
          style={{ color: 'var(--ink-dim)' }}
          data-testid="tiers-not-configured"
        >
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
          Hold the configured CL8Y CW20 and register for a tier that matches your balance. Holding alone does not apply
          a discount.
        </p>
      </div>

      {address && (
        <div className="shell-panel-strong mb-6" data-testid="tiers-your-status">
          <div className="flex items-center justify-between gap-3">
            <div>
              <p className="label-glass">Your Status</p>
              {registration?.registered ? (
                <div>
                  <span className="font-medium" style={{ color: 'var(--ink)' }}>
                    {tierTitle(currentTierId ?? 0, registration.tier?.governance_only === true)}
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
                type="button"
                data-testid="tiers-deregister"
                onClick={() => {
                  sounds.playButtonPress()
                  deregisterMutation.mutate()
                }}
                disabled={deregisterMutation.isPending}
                className="btn-muted !text-xs min-h-11 disabled:opacity-50"
              >
                {deregisterMutation.isPending ? 'Deregistering...' : 'Deregister'}
              </button>
            )}
          </div>
          {(registerMutation.isError || deregisterMutation.isError) && (
            <div className="mt-3 alert-error !text-xs">
              {humanizeUserFacingErrorFromUnknown(
                registerMutation.isError ? registerMutation.error : deregisterMutation.error
              )}
            </div>
          )}
        </div>
      )}

      {!address && (
        <div
          className="shell-panel-strong mb-6 text-center text-sm"
          style={{ color: 'var(--ink-dim)' }}
          data-testid="tiers-connect-banner"
        >
          Connect your wallet to register for a fee discount tier.
        </div>
      )}

      {tiersQuery.isLoading && (
        <div className="shell-panel-strong flex items-center justify-center gap-3 py-8" aria-live="polite">
          <Spinner /> <span style={{ color: 'var(--ink-dim)' }}>Loading tiers...</span>
        </div>
      )}

      {tiersQuery.isError && (
        <RetryError
          data-testid="tiers-retry-error"
          message={`Failed to load tiers: ${tiersQuery.error?.message ?? 'Unknown error'}`}
          onRetry={() => void tiersQuery.refetch()}
        />
      )}

      <div className="space-y-3" data-testid="tiers-card-list">
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

      <div className="mt-8 shell-panel-strong" data-testid="tiers-how-it-works">
        <h3 className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading" style={{ color: 'var(--ink)' }}>
          How it works
        </h3>
        <div className="text-sm space-y-2" style={{ color: 'var(--ink-dim)' }}>
          <p>
            Your swap fee is reduced based on your registered tier. Limit-order <em>placement</em> uses a deeper
            discount (tier 9 is 0). Taking or swapping still uses the swap discount. You must hold the fee-discount
            contract&apos;s configured CL8Y CW20 (
            <span className="font-mono text-xs">{CL8Y_TOKEN_ADDRESS || 'not configured'}</span>) and register on this
            page — other similarly named tokens do not count. If you drop below the required holding at any time, you
            lose your tier.
          </p>
          <p className="text-xs" style={{ color: 'var(--ink-subtle)' }}>
            The default base fee is {(baseFee / 100).toFixed(1)}% for most pairs. Some pairs may have a different base
            fee &mdash; your tier discount applies as a percentage off whichever base fee the pair uses.
          </p>
          <HowItWorksLadder selfRegisterTiers={selfRegisterTiers} baseFee={baseFee} />
          <p className="text-xs mt-2" style={{ color: 'var(--ink-subtle)' }}>
            *Effective fee is the swap / book-take rate at the default {(baseFee / 100).toFixed(1)}% base. Limit place
            is half of the limit-order discount (tier 9 is 0). Crossing the book still charges the taker&apos;s swap
            fee.
          </p>
        </div>
      </div>
    </div>
  )
}
