import { Link } from 'react-router-dom'
import { AddressRow, CopyButton } from '@/components/ui'
import {
  MIGRATE_CREATE_PAIR_HINT,
  MIGRATE_REGISTER_READY,
  MIGRATE_REGISTER_UNLISTED_OTHER,
  MIGRATE_REGISTER_WAIT,
  MIGRATE_SUCCESS_CHECKLIST,
  MIGRATE_VENUE_CL8Y_EMPTY,
  MIGRATE_VENUE_CL8Y_PAUSE,
  MIGRATE_VENUE_GDEX,
  MIGRATE_VENUE_OTHER_DEX,
  MIGRATE_VENUE_TERRAPORT_INCOMPLETE,
  buildGovernanceTicket,
  registerCtaState,
  type Cl8yVenueRow,
  type MigratePairInventory,
  type OtherDexVenueRow,
  type RegisterCtaContext,
} from '@/utils/communityTaxMigratePairs'
import { shortPairAddr } from '@/utils/communityTaxRegisterPair'

export type MigratePairInventoryCardProps = {
  inventory: MigratePairInventory
  token: string
  phase: 'confirm' | 'success' | 'readonly'
  registerCtx: RegisterCtaContext
  onRegister?: (pair: string) => void
  registeringPair?: string | null
}

function Cl8yRow({
  row,
  registerCtx,
  onRegister,
  registeringPair,
  showRegister,
}: {
  row: Cl8yVenueRow
  registerCtx: RegisterCtaContext
  onRegister?: (pair: string) => void
  registeringPair?: string | null
  showRegister: boolean
}) {
  const cta = showRegister ? registerCtaState(row, registerCtx) : 'hidden'
  return (
    <li className="space-y-1" data-testid="migrate-venue-cl8y-row">
      <div className="flex flex-wrap items-center gap-2 text-sm">
        <span>
          {row.symbols[0]}/{row.symbols[1]}
        </span>
        <AddressRow
          address={row.pair}
          copyAriaLabel="Copy CL8Y pool address"
          explorerAriaLabel="View CL8Y pool on explorer"
          data-testid="migrate-venue-cl8y-addr"
        />
      </div>
      <p className="text-xs" style={{ color: 'var(--ink-dim)' }}>
        {row.otherAssetListed === false ? MIGRATE_REGISTER_UNLISTED_OTHER : MIGRATE_VENUE_CL8Y_PAUSE}
      </p>
      {cta === 'ready' && onRegister && (
        <button
          type="button"
          className="btn-primary w-full"
          data-testid="migrate-register-cl8y"
          disabled={registeringPair === row.pair}
          onClick={() => onRegister(row.pair)}
        >
          {registeringPair === row.pair ? 'Registering…' : `${MIGRATE_REGISTER_READY} · ${shortPairAddr(row.pair)}`}
        </button>
      )}
      {cta === 'wait_refresh' && (
        <p className="text-xs" style={{ color: 'var(--ink-dim)' }} data-testid="migrate-register-wait">
          {MIGRATE_REGISTER_WAIT}
        </p>
      )}
    </li>
  )
}

function OtherDexRow({ row }: { row: OtherDexVenueRow }) {
  const addr = row.pair || row.pairDisplay
  return (
    <li className="space-y-1" data-testid="migrate-venue-other-row">
      <div className="flex flex-wrap items-center gap-2 text-sm">
        <span>
          {row.symbols[0]}/{row.symbols[1]}
        </span>
        {row.pair ? (
          <AddressRow
            address={row.pair}
            copyAriaLabel="Copy other DEX pool address"
            explorerAriaLabel="View other DEX pool on explorer"
            data-testid="migrate-venue-other-addr"
          />
        ) : addr ? (
          <span className="font-mono text-xs" data-testid="migrate-venue-other-display">
            {addr}
          </span>
        ) : null}
      </div>
    </li>
  )
}

/** Confirm / success / read-only venue list. No Refresh / pause / whitelist controls. */
export function MigratePairInventoryCard({
  inventory,
  token,
  phase,
  registerCtx,
  onRegister,
  registeringPair,
}: MigratePairInventoryCardProps) {
  const showRegister = phase === 'success' || (phase === 'readonly' && registerCtx.postAdopt)
  const ticket = buildGovernanceTicket(token, inventory.cl8y)

  return (
    <div className="card-glass space-y-3 p-3" data-testid="migrate-venue-inventory">
      {phase === 'success' && (
        <ol className="list-decimal pl-4 text-sm space-y-1" data-testid="migrate-success-checklist">
          {MIGRATE_SUCCESS_CHECKLIST.map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ol>
      )}

      <div data-testid="migrate-venue-cl8y">
        <p className="text-sm font-semibold mb-1">CL8Y pools</p>
        {inventory.cl8y.length === 0 ? (
          <div className="space-y-1" data-testid="migrate-venue-cl8y-empty">
            <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
              {MIGRATE_VENUE_CL8Y_EMPTY}
            </p>
            <p className="text-xs" style={{ color: 'var(--ink-dim)' }}>
              {MIGRATE_CREATE_PAIR_HINT}{' '}
              <Link className="underline" to="/create" data-testid="migrate-create-pair">
                Create Pair
              </Link>
            </p>
          </div>
        ) : (
          <ul className="space-y-3">
            {inventory.cl8y.map((row) => (
              <Cl8yRow
                key={row.pair}
                row={row}
                registerCtx={registerCtx}
                onRegister={onRegister}
                registeringPair={registeringPair}
                showRegister={showRegister}
              />
            ))}
          </ul>
        )}
      </div>

      <div data-testid="migrate-venue-other">
        <p className="text-sm font-semibold mb-1">Other DEX</p>
        <p className="text-xs mb-2" style={{ color: 'var(--ink-dim)' }}>
          {MIGRATE_VENUE_OTHER_DEX}
        </p>
        {inventory.otherDex.length > 0 && (
          <ul className="space-y-2">
            {inventory.otherDex.map((row, i) => (
              <OtherDexRow key={`${row.pair || row.pairDisplay || i}`} row={row} />
            ))}
          </ul>
        )}
        {inventory.terraportIncomplete && (
          <p
            className="text-xs mt-2"
            style={{ color: 'var(--ink-dim)' }}
            data-testid="migrate-venue-terraport-incomplete"
          >
            {MIGRATE_VENUE_TERRAPORT_INCOMPLETE}
          </p>
        )}
        <p className="text-xs mt-2" style={{ color: 'var(--ink-dim)' }} data-testid="migrate-venue-gdex">
          {MIGRATE_VENUE_GDEX}
        </p>
      </div>

      {inventory.cl8y.length > 0 && (
        <CopyButton
          text={ticket}
          ariaLabel="Copy details for governance"
          buttonLabel="Copy details for governance"
          data-testid="migrate-governance-copy"
        />
      )}
    </div>
  )
}
