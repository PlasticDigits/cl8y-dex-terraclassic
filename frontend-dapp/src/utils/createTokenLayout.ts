/**
 * GitLab #669 — Create Token desktop density (layout only).
 * Configured wizard uses the `.app-main` (1080px) budget at `md+`.
 * Phone (`<md` / ≤767px) stays a single column. Do not reuse on Swap / Create Pair.
 */

export const CREATE_TOKEN_PAGE_CLASS = 'w-full'
export const CREATE_TOKEN_UNAVAILABLE_CLASS = 'max-w-[520px] mx-auto'

export const CREATE_TOKEN_DESKTOP_GRID_CLASS = 'grid grid-cols-1 md:grid-cols-2 gap-6 min-w-0'
export const CREATE_TOKEN_IDENTITY_ROW_CLASS =
  'grid grid-cols-1 min-w-0 md:grid-cols-[minmax(0,2fr)_minmax(0,2fr)_minmax(0,1fr)] gap-3'
export const CREATE_TOKEN_TAX_ROW_CLASS = 'grid grid-cols-2 gap-3 min-w-0'
export const CREATE_TOKEN_WALLET_ROW_CLASS = 'grid grid-cols-1 min-w-0 md:grid-cols-2 gap-3'
export const CREATE_TOKEN_SKU_GRID_CLASS = 'grid grid-cols-1 md:grid-cols-2 gap-2'
export const CREATE_TOKEN_VARIABLE_ROW_CLASS = 'grid grid-cols-1 md:grid-cols-3 gap-3 min-w-0'
export const CREATE_TOKEN_AUTOLP_ROW_CLASS = 'grid grid-cols-1 md:grid-cols-2 gap-3 min-w-0'
export const CREATE_TOKEN_GUARDS_ROW_CLASS = 'grid grid-cols-1 md:grid-cols-2 gap-3 min-w-0'
export const CREATE_TOKEN_SINK_ROW_CLASS = 'grid grid-cols-1 sm:grid-cols-3 gap-2 min-w-0'

export const CREATE_TOKEN_BECH32_INPUT_CLASS = 'input-glass w-full min-w-0 break-all'
export const CREATE_TOKEN_SKU_LABEL_CLASS = 'flex items-start gap-2 text-sm min-h-11'
export const CREATE_TOKEN_ACK_LABEL_CLASS = 'flex items-start gap-2 text-sm min-h-11'
