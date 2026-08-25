/**
 * Columbus-5 CW20 pins for Keplr contract-registry recognition (GitLab #629).
 *
 * Job 1 is name + logo in Keplr Add Token. Job 2 (USD) is a separate CoinGecko /
 * Keplr price path — do not invent `price`, `priceUrl`, or `oracle` fields.
 */

export const KEPLR_CONTRACT_REGISTRY_REPO = 'https://github.com/chainapsis/keplr-contract-registry'

/** Keplr chain folder for Terra Classic. Not `terra` and not phoenix-1. */
export const KEPLR_CW20_CHAIN_DIR = 'columbus' as const

export const KEPLR_CW20_CHAIN_ID = 'columbus-5' as const

export const KEPLR_CW20_IMAGE_BASE =
  'https://raw.githubusercontent.com/chainapsis/keplr-contract-registry/main/images/columbus'

export const CL8Y_COINGECKO_ID = 'ceramicliberty-com' as const

export type KeplrCw20SubmitStatus = 'submit' | 'already_registered'

export type KeplrCw20CatalogEntry = {
  symbol: string
  name: string
  decimals: number
  contractAddress: string
  imageFile: string
  sourceImage: string
  status: KeplrCw20SubmitStatus
  /** Only when CoinGecko lists the economic asset. Omit otherwise (K629-4). */
  coinGeckoId?: string
}

export type KeplrCw20TokenJson = {
  contractAddress: string
  imageUrl: string
  metadata: {
    name: string
    symbol: string
    decimals: number
  }
  coinGeckoId?: string
}

/**
 * Permanent CMM / DEX CW20s only. Gems, ALPHA, USTRIX, SpaceUSD, and community-tax
 * templates stay out (K629-2).
 */
export const KEPLR_CW20_CATALOG: readonly KeplrCw20CatalogEntry[] = [
  {
    symbol: 'CL8Y',
    name: 'CL8Y',
    decimals: 18,
    contractAddress: 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3',
    imageFile: 'CL8Y.png',
    sourceImage: 'tokenlist/images/CL8Y.png',
    status: 'submit',
    coinGeckoId: CL8Y_COINGECKO_ID,
  },
  {
    symbol: 'UST1',
    name: 'UST1',
    decimals: 6,
    contractAddress: 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72',
    imageFile: 'UST1.png',
    sourceImage: 'tokenlist/images/UST1.png',
    status: 'submit',
  },
  {
    // Live Keplr file already uses this name + 18 decimals. Do not rename in-pack.
    symbol: 'USTR',
    name: 'USTC Repeg',
    decimals: 18,
    contractAddress: 'terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv',
    imageFile: 'USTR.png',
    sourceImage: 'tokenlist/images/USTR.png',
    status: 'already_registered',
  },
  {
    symbol: 'cLUNC',
    name: 'Wrapped Luna Classic',
    decimals: 6,
    contractAddress: 'terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg',
    imageFile: 'CLUNC.png',
    sourceImage: 'tokenlist/images/CLUNC.png',
    status: 'submit',
  },
  {
    symbol: 'cUSTC',
    name: 'Wrapped TerraClassicUSD',
    decimals: 6,
    contractAddress: 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch',
    imageFile: 'CUSTC.png',
    sourceImage: 'tokenlist/images/CUSTC.png',
    status: 'submit',
  },
  {
    symbol: 'vFDUSD',
    name: 'Venus FDUSD (bridged)',
    decimals: 6,
    contractAddress: 'terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3',
    imageFile: 'VFDUSD.png',
    sourceImage: 'tokenlist/images/VFDUSD.png',
    status: 'submit',
  },
]

const FORBIDDEN_PRICE_KEYS = ['price', 'priceUrl', 'oracle', 'marketId'] as const

export function keplrCw20ImageUrl(imageFile: string): string {
  return `${KEPLR_CW20_IMAGE_BASE}/${imageFile}`
}

export function keplrCw20TokenFilename(contractAddress: string): string {
  return `${contractAddress}.json`
}

export function buildKeplrCw20TokenJson(entry: KeplrCw20CatalogEntry): KeplrCw20TokenJson {
  const payload: KeplrCw20TokenJson = {
    contractAddress: entry.contractAddress,
    imageUrl: keplrCw20ImageUrl(entry.imageFile),
    metadata: {
      name: entry.name,
      symbol: entry.symbol,
      decimals: entry.decimals,
    },
  }
  if (entry.coinGeckoId) {
    payload.coinGeckoId = entry.coinGeckoId
  }
  return payload
}

export function keplrCw20TokensToSubmit(): readonly KeplrCw20CatalogEntry[] {
  return KEPLR_CW20_CATALOG.filter((t) => t.status === 'submit')
}

export function isKeplrRecognitionCw20(address: string): boolean {
  const lower = address.toLowerCase()
  return KEPLR_CW20_CATALOG.some((t) => t.contractAddress === lower)
}

export function lookupKeplrCw20(address: string): KeplrCw20CatalogEntry | undefined {
  const lower = address.toLowerCase()
  return KEPLR_CW20_CATALOG.find((t) => t.contractAddress === lower)
}

/** README omitted this field; live columbus tokens (e.g. MIR) already set it. */
export function keplrCw20AllowsCoinGeckoId(): boolean {
  return true
}

export function keplrCw20ForbiddenPriceKeys(): readonly string[] {
  return FORBIDDEN_PRICE_KEYS
}
