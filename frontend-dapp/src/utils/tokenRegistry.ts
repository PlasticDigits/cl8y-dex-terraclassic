import type { AssetInfo } from '@/types'
import { LUNC_C_TOKEN_ADDRESS, UST1_TOKEN_ADDRESS, USTC_C_TOKEN_ADDRESS, VFDUSD_TOKEN_ADDRESS } from './constants'
import { foldTokenlistSymbol, queryTokenToExecuteId } from './tokenlistQueryCatalog'

export interface TokenRegistryEntry {
  symbol: string
  name: string
  decimals: number
  logoURI: string
  type: 'native' | 'cw20'
}

export const TOKENS: TokenRegistryEntry[] = [
  {
    symbol: 'LUNC',
    name: 'Terra Luna Classic',
    decimals: 6,
    type: 'native',
    logoURI: 'https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/raw/main/tokenlist/images/LUNC.png',
  },
  {
    symbol: 'USTC',
    name: 'TerraClassicUSD',
    decimals: 6,
    type: 'native',
    logoURI: 'https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/raw/main/tokenlist/images/USTC.png',
  },
  {
    symbol: 'CL8Y',
    name: 'CL8Y Token',
    // On-chain CL8Y / LocalTerra TCL8Y use 18 decimals (GitLab #476 / #383).
    decimals: 18,
    type: 'cw20',
    logoURI: 'https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/raw/main/tokenlist/images/CL8Y.png',
  },
  {
    symbol: 'USTR',
    name: 'USTR Token',
    decimals: 18,
    type: 'cw20',
    logoURI: 'https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/raw/main/tokenlist/images/USTR.png',
  },
  {
    symbol: 'ALPHA',
    name: 'Alpha Token',
    decimals: 6,
    type: 'cw20',
    logoURI: 'https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/raw/main/tokenlist/images/ALPHA.png',
  },
  {
    symbol: 'USTRIX',
    name: 'USTRIX Token',
    decimals: 6,
    type: 'cw20',
    logoURI: 'https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/raw/main/tokenlist/images/USTRIX.png',
  },
  {
    symbol: 'SpaceUSD',
    name: 'SpaceUSD Token',
    decimals: 6,
    type: 'cw20',
    logoURI: 'https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/raw/main/tokenlist/images/SPACEUSD.png',
  },
  {
    // Product symbols cLUNC / cUSTC (GitLab #507); env keys remain VITE_LUNC_C_* for LocalTerra parity.
    // Logos = base LUNC/USTC art + blue "c" badge (tokenlist/images/CLUNC.png, CUSTC.png).
    symbol: 'cLUNC',
    name: 'Wrapped Luna Classic',
    decimals: 6,
    type: 'cw20',
    logoURI: 'https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/raw/main/tokenlist/images/CLUNC.png',
  },
  {
    symbol: 'cUSTC',
    name: 'Wrapped TerraClassicUSD',
    decimals: 6,
    type: 'cw20',
    logoURI: 'https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/raw/main/tokenlist/images/CUSTC.png',
  },
  {
    symbol: 'UST1',
    name: 'UST1',
    decimals: 6,
    type: 'cw20',
    logoURI: 'https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/raw/main/tokenlist/images/UST1.png',
  },
  {
    symbol: 'vFDUSD',
    name: 'Venus FDUSD (bridged)',
    decimals: 6,
    type: 'cw20',
    logoURI: 'https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/raw/main/tokenlist/images/VFDUSD.png',
  },
]

// Allowlist only — unknown bank/IBC denoms stay raw (GitLab #630).
const DENOM_MAP: Record<string, string> = {
  uluna: 'LUNC',
  uusd: 'USTC',
}

const CW20_MAP: Record<string, string> = {
  terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3: 'CL8Y',
  terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv: 'USTR',
  terra1x6e64es6yhauhvs3prvpdg2gkqdtfru840wgnhs935x8axr7zxkqzysuxz: 'ALPHA',
  terra1r3eaa2tucjr3es88wzuqpgxvssqflk9cghrjmf9uneds8wljyapqwtrcp5: 'USTRIX',
  terra1cvd5cgrs8rrl96hte34n57497u5f9cwuv3e6ztxgetkx4uzmcdyswv79zl: 'SpaceUSD',
}

if (LUNC_C_TOKEN_ADDRESS) CW20_MAP[LUNC_C_TOKEN_ADDRESS.toLowerCase()] = 'cLUNC'
if (USTC_C_TOKEN_ADDRESS) CW20_MAP[USTC_C_TOKEN_ADDRESS.toLowerCase()] = 'cUSTC'
if (UST1_TOKEN_ADDRESS) CW20_MAP[UST1_TOKEN_ADDRESS.toLowerCase()] = 'UST1'
if (VFDUSD_TOKEN_ADDRESS) CW20_MAP[VFDUSD_TOKEN_ADDRESS.toLowerCase()] = 'vFDUSD'

// Published columbus-5 addresses (GitLab #506 / #507) — always resolve logos even before env is set.
CW20_MAP['terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72'] = 'UST1'
CW20_MAP['terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3'] = 'vFDUSD'
CW20_MAP['terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg'] = 'cLUNC'
CW20_MAP['terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch'] = 'cUSTC'

const BY_SYMBOL = new Map<string, TokenRegistryEntry>()
for (const t of TOKENS) BY_SYMBOL.set(t.symbol, t)

export function lookupByDenom(denom: string): TokenRegistryEntry | undefined {
  const sym = DENOM_MAP[denom.toLowerCase()]
  return sym ? BY_SYMBOL.get(sym) : undefined
}

export function lookupByCW20(addr: string): TokenRegistryEntry | undefined {
  const sym = CW20_MAP[addr.toLowerCase()]
  return sym ? BY_SYMBOL.get(sym) : undefined
}

export function lookupByTokenId(tokenId: string): TokenRegistryEntry | undefined {
  return lookupByDenom(tokenId) ?? lookupByCW20(tokenId)
}

/**
 * Product ticker → execute id for Swap deep links (#711 / #715). Case-insensitive.
 * Tokenlist symbols (overlay address on LocalTerra) plus inbound-only `UST` → `uusd`.
 * Unknown tickers (including gems and EVM names) → `undefined`.
 */
export function lookupTokenIdByProductTicker(raw: string): string | undefined {
  const trimmed = raw.trim()
  if (!trimmed) return undefined
  const folded = foldTokenlistSymbol(trimmed)
  if (folded === 'ust') return 'uusd'
  if (folded === 'uluna' || folded === 'uusd') return folded
  return queryTokenToExecuteId(trimmed) ?? undefined
}

/**
 * Static product ticker for known natives (`uluna`→LUNC, `uusd`→USTC) and listed CW20s
 * (cLUNC / cUSTC / UST1 / …). Undefined for unknown bank denoms (fail closed, GitLab #630).
 */
export function registryProductSymbol(tokenId: string | undefined | null): string | undefined {
  if (!tokenId?.trim()) return undefined
  return lookupByTokenId(tokenId)?.symbol
}

export function lookupByAssetInfo(info: AssetInfo): TokenRegistryEntry | undefined {
  if ('token' in info) return lookupByCW20(info.token.contract_addr)
  return lookupByDenom(info.native_token.denom)
}
