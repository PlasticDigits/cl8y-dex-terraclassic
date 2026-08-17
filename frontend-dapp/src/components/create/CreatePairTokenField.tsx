import { useId, useState } from 'react'
import { TokenSearchSelect } from '@/components/trade/TokenSearchSelect'
import { listedCreatePairAddress } from '@/utils/createPairTokenCatalog'
import { getTerraAddressInputError } from '@/utils/terraAddressValidation'

export type CreatePairCodeIdCheck = {
  valid: boolean
  reason: string | null
} | null

export type CreatePairTokenFieldProps = {
  label: string
  selectAriaLabel: string
  value: string
  onChange: (address: string) => void
  catalog: readonly string[]
  excludeToken?: string
  codeIdCheck?: CreatePairCodeIdCheck
}

/**
 * Listed-CW20 combobox + progressive-disclosure custom `terra1…` paste (GitLab #542).
 * Picker `onChange` only accepts ids in `catalog`. Custom paste stays permissionless.
 */
export function CreatePairTokenField({
  label,
  selectAriaLabel,
  value,
  onChange,
  catalog,
  excludeToken,
  codeIdCheck,
}: CreatePairTokenFieldProps) {
  const comboboxId = useId()
  const customId = useId()
  const customPanelId = useId()
  const [customOpen, setCustomOpen] = useState(false)

  const tokens = [...catalog]
  const excludeListed = excludeToken ? listedCreatePairAddress(tokens, excludeToken) : undefined
  const listedValue = listedCreatePairAddress(tokens, value) ?? ''
  const addressError = getTerraAddressInputError(value)

  const selectListed = (id: string) => {
    const listed = listedCreatePairAddress(tokens, id)
    if (!listed) return
    onChange(listed)
    setCustomOpen(false)
  }

  return (
    <div data-testid={`create-pair-field-${label.replace(/\s+/g, '-').toLowerCase()}`}>
      <label className="label-glass" htmlFor={comboboxId}>
        {label}
      </label>
      <TokenSearchSelect
        id={comboboxId}
        aria-label={selectAriaLabel}
        value={listedValue}
        tokens={tokens}
        excludeToken={excludeListed}
        onChange={selectListed}
        className="relative w-full"
        placeholder="Select token"
      />

      <button
        type="button"
        className="mt-2 text-xs font-semibold uppercase tracking-wide"
        style={{ color: 'var(--ink-dim)' }}
        aria-expanded={customOpen}
        aria-controls={customPanelId}
        data-testid={`create-pair-custom-toggle-${label.replace(/\s+/g, '-').toLowerCase()}`}
        onClick={() => setCustomOpen((open) => !open)}
      >
        Custom contract
      </button>

      {customOpen ? (
        <div id={customPanelId} className="mt-2">
          <label className="label-glass" htmlFor={customId}>
            {label} Contract Address
          </label>
          <input
            id={customId}
            type="text"
            value={value}
            onChange={(e) => onChange(e.target.value)}
            placeholder="terra1..."
            className="input-glass font-mono"
            data-testid={`create-pair-custom-address-${label.replace(/\s+/g, '-').toLowerCase()}`}
            autoComplete="off"
            spellCheck={false}
          />
        </div>
      ) : null}

      {addressError ? (
        <p className="text-red-400 text-xs mt-1 uppercase tracking-wide font-semibold">{addressError}</p>
      ) : null}
      {codeIdCheck && !codeIdCheck.valid ? (
        <p className="text-amber-400 text-xs mt-1 uppercase tracking-wide font-semibold">{codeIdCheck.reason}</p>
      ) : null}
      {codeIdCheck?.valid ? (
        <p className="text-green-400 text-xs mt-1 uppercase tracking-wide font-semibold">Code ID whitelisted</p>
      ) : null}
    </div>
  )
}
