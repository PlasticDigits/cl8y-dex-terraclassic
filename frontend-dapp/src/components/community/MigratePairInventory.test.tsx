import type { ComponentProps } from 'react'
import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router-dom'
import { MigratePairInventoryCard } from './MigratePairInventory'
import {
  ALPHA_TERRAPORT_LUNC_PAIR,
  ALPHA_TERRAPORT_USTC_PAIR,
  MIGRATE_REGISTER_READY,
  MIGRATE_REGISTER_WAIT,
  MIGRATE_VENUE_CL8Y_EMPTY,
  MIGRATE_VENUE_GDEX,
  MIGRATE_VENUE_OTHER_DEX,
  type MigratePairInventory,
} from '@/utils/communityTaxMigratePairs'

const TOKEN = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const CL8Y = 'terra1aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'

const readyCtx = { postAdopt: true, isManager: true, taxPinMatches: true, adminIsCmm: true }
const hiddenCtx = { postAdopt: false, isManager: true, taxPinMatches: false, adminIsCmm: false }

const emptyInv: MigratePairInventory = { cl8y: [], otherDex: [], terraportIncomplete: false }

const alphaInv: MigratePairInventory = {
  cl8y: [],
  otherDex: [
    { venue: 'other_dex', pair: ALPHA_TERRAPORT_LUNC_PAIR, symbols: ['ALPHA', 'LUNC'], source: 'static' },
    { venue: 'other_dex', pair: ALPHA_TERRAPORT_USTC_PAIR, symbols: ['ALPHA', 'USTC'], source: 'static' },
  ],
  terraportIncomplete: false,
}

const cl8yInv: MigratePairInventory = {
  cl8y: [
    {
      venue: 'cl8y',
      pair: CL8Y,
      symbols: ['GEM', 'LUNC'],
      otherAssetLabel: 'LUNC',
      otherAssetListed: true,
      frozen: true,
      registered: false,
      factoryVerified: true,
    },
  ],
  otherDex: [],
  terraportIncomplete: false,
}

function renderCard(
  over: Partial<ComponentProps<typeof MigratePairInventoryCard>> & { inventory: MigratePairInventory }
) {
  return render(
    <MemoryRouter>
      <MigratePairInventoryCard token={TOKEN} phase="confirm" registerCtx={hiddenCtx} {...over} />
    </MemoryRouter>
  )
}

describe('MigratePairInventoryCard (#634)', () => {
  it('empty CL8Y + GDEX instruction + Create Pair link, no register', () => {
    renderCard({ inventory: emptyInv })
    expect(screen.getByTestId('migrate-venue-cl8y-empty')).toHaveTextContent(MIGRATE_VENUE_CL8Y_EMPTY)
    expect(screen.getByTestId('migrate-create-pair')).toHaveAttribute('href', '/create')
    expect(screen.getByTestId('migrate-venue-gdex')).toHaveTextContent(MIGRATE_VENUE_GDEX)
    expect(screen.getByTestId('migrate-venue-other')).toHaveTextContent(MIGRATE_VENUE_OTHER_DEX)
    expect(screen.queryByTestId('migrate-register-cl8y')).not.toBeInTheDocument()
  })

  it('ALPHA Terraport rows never get a register button', () => {
    renderCard({ inventory: alphaInv, phase: 'success', registerCtx: readyCtx, onRegister: vi.fn() })
    expect(screen.getAllByTestId('migrate-venue-other-row')).toHaveLength(2)
    expect(screen.getByText(/ALPHA\/LUNC/)).toBeInTheDocument()
    expect(screen.getByText(/ALPHA\/USTC/)).toBeInTheDocument()
    expect(screen.queryByTestId('migrate-register-cl8y')).not.toBeInTheDocument()
  })

  it('frozen CL8Y after adopt shows wait copy, no enabled register', () => {
    renderCard({ inventory: cl8yInv, phase: 'success', registerCtx: readyCtx, onRegister: vi.fn() })
    expect(screen.getByTestId('migrate-register-wait')).toHaveTextContent(MIGRATE_REGISTER_WAIT)
    expect(screen.queryByTestId('migrate-register-cl8y')).not.toBeInTheDocument()
    expect(screen.queryByText(/refresh_pair_asset_code_ids|RegisterListedPair|VITE_/)).not.toBeInTheDocument()
  })

  it('unfrozen CL8Y after adopt offers per-row register, not highest-LP', async () => {
    const user = userEvent.setup()
    const onRegister = vi.fn()
    const unfrozen = {
      ...cl8yInv,
      cl8y: [{ ...cl8yInv.cl8y[0], frozen: false as const }],
    }
    renderCard({ inventory: unfrozen, phase: 'success', registerCtx: readyCtx, onRegister })
    const btn = screen.getByTestId('migrate-register-cl8y')
    expect(btn).toHaveTextContent(MIGRATE_REGISTER_READY)
    expect(btn).not.toHaveTextContent(/largest/i)
    await user.click(btn)
    expect(onRegister).toHaveBeenCalledWith(CL8Y)
  })

  it('success checklist is present; confirm phase has no checklist', () => {
    const { rerender } = renderCard({ inventory: emptyInv, phase: 'confirm' })
    expect(screen.queryByTestId('migrate-success-checklist')).not.toBeInTheDocument()
    rerender(
      <MemoryRouter>
        <MigratePairInventoryCard token={TOKEN} inventory={emptyInv} phase="success" registerCtx={readyCtx} />
      </MemoryRouter>
    )
    expect(screen.getByTestId('migrate-success-checklist')).toBeInTheDocument()
  })

  it('Terraport incomplete copy does not hide the card', () => {
    renderCard({ inventory: { ...emptyInv, terraportIncomplete: true } })
    expect(screen.getByTestId('migrate-venue-terraport-incomplete')).toBeInTheDocument()
    expect(screen.getByTestId('migrate-venue-inventory')).toBeInTheDocument()
  })
})
