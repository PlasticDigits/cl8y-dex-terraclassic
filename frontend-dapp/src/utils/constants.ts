export const TERRA_LCD_URL = import.meta.env.VITE_TERRA_LCD_URL || 'https://terra-classic-lcd.publicnode.com'
export const TERRA_RPC_URL = import.meta.env.VITE_TERRA_RPC_URL || 'https://terra-classic-rpc.publicnode.com:443'
export const FACTORY_CONTRACT_ADDRESS = import.meta.env.VITE_FACTORY_ADDRESS || ''
export const ROUTER_CONTRACT_ADDRESS = import.meta.env.VITE_ROUTER_ADDRESS || ''
export const FEE_DISCOUNT_CONTRACT_ADDRESS = import.meta.env.VITE_FEE_DISCOUNT_ADDRESS || ''
export const CL8Y_TOKEN_ADDRESS =
  import.meta.env.VITE_CL8Y_TOKEN_ADDRESS || 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
export const WRAP_MAPPER_CONTRACT_ADDRESS = import.meta.env.VITE_WRAP_MAPPER_ADDRESS || ''
export const TREASURY_CONTRACT_ADDRESS = import.meta.env.VITE_TREASURY_ADDRESS || ''
export const LUNC_C_TOKEN_ADDRESS = import.meta.env.VITE_LUNC_C_TOKEN_ADDRESS || ''
export const USTC_C_TOKEN_ADDRESS = import.meta.env.VITE_USTC_C_TOKEN_ADDRESS || ''

/** Default-branch docs in GitLab (security audit, limit orders, ADRs). */
export const DOCS_GITLAB_BASE = 'https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/blob/main/docs'

export const NATIVE_WRAPPED_PAIRS: Record<string, string> = {
  uluna: LUNC_C_TOKEN_ADDRESS,
  uusd: USTC_C_TOKEN_ADDRESS,
}

export const WRAPPED_NATIVE_PAIRS: Record<string, string> = {
  [LUNC_C_TOKEN_ADDRESS]: 'uluna',
  [USTC_C_TOKEN_ADDRESS]: 'uusd',
}

export const WRAP_GAS_LIMIT = 300000
export const UNWRAP_GAS_LIMIT = 400000

export const DEV_MODE = import.meta.env.VITE_DEV_MODE === 'true'

/** Matches Station `gasPriceStep` / Terra Classic norms; used as a floor so fee math cannot underpay at broadcast (GitLab #127). */
export const MIN_GAS_PRICE_ULUNA = 28.325

const DEFAULT_GAS_PRICE_ULUNA = String(MIN_GAS_PRICE_ULUNA)

export const GAS_PRICE_ULUNA = import.meta.env.VITE_GAS_PRICE_ULUNA || DEFAULT_GAS_PRICE_ULUNA

/**
 * Gas price (uluna per gas unit) used for `Fee.amount` and wallet `gasPrice`.
 * Never below {@link MIN_GAS_PRICE_ULUNA}: a too-low `VITE_GAS_PRICE_ULUNA` causes **insufficient fees**
 * while `gas_wanted` is still high (e.g. `increase_allowance` before limit placement).
 */
export function effectiveGasPriceUluna(): number {
  const parsed = parseFloat(GAS_PRICE_ULUNA)
  const base = Number.isFinite(parsed) && parsed > 0 ? parsed : MIN_GAS_PRICE_ULUNA
  return Math.max(base, MIN_GAS_PRICE_ULUNA)
}
export const SWAP_GAS_PER_HOP = 600000
/**
 * Multiplier on (per-hop base × hop count) before floor/padding.
 * Match operational `--gas-adjustment 1.3` in `deploy-dex-local.sh` / `terrad` so the dApp is not tighter than CLI defaults.
 * GitLab #115: pool-only swap used ~753,321 gas; a 1.1× buffer produced 710k wanted and failed on-chain.
 */
export const SWAP_GAS_BUFFER = 1.3
/**
 * Minimum gas attributed per hop for `execute_swap_operations` (total floor = hops × this).
 * Guards against underestimates when buffer × base is still too low for some pairs.
 */
export const EXECUTE_SWAP_OPS_MIN_GAS_PER_HOP = 661000
/**
 * Extra gas added per hop on top of the buffered estimate (absorbs small runtime variance;
 * e.g. QA saw gasUsed 1,320,097 vs wanted 1,320,000 on a 2-hop).
 */
export const SWAP_MULTIHOP_GAS_PADDING_PER_HOP = 50000
/**
 * Flat headroom on top of buffered + padded swap gas so `gasUsed` cannot exceed `gasWanted`
 * by a few hundred units at the ceiling (GitLab #115 follow-up: 830,102 used vs 830,000 wanted).
 */
export const SWAP_GAS_SAFETY_MARGIN = 10000

/**
 * Direct pair Pattern C hybrid gas (GitLab #249; tuned after #248 transfer aggregation / #252 benchmarks).
 * `gasWanted ≈ HYBRID_SWAP_BASE_GAS + HYBRID_SWAP_PER_MAKER_GAS × (makersUsed + HYBRID_SWAP_MAKER_GAS_BUFFER)`,
 * clamped to [`HYBRID_SWAP_GAS_FLOOR`, `HYBRID_SWAP_GAS_LIMIT`]. Pool-only leg (`book_input = 0`) uses
 * {@link gasLimitForExecuteSwapOperations}(1) instead.
 */
export const HYBRID_SWAP_BASE_GAS = 550_000
export const HYBRID_SWAP_PER_MAKER_GAS = 65_000
export const HYBRID_SWAP_MAKER_GAS_BUFFER = 2
export const HYBRID_SWAP_GAS_FLOOR = 600_000

type NetworkConfig = {
  terra: {
    chainId: string
    lcd: string
    rpc: string
  }
}

export const NETWORKS: Record<string, NetworkConfig> = {
  local: {
    terra: {
      chainId: 'localterra',
      lcd: import.meta.env.VITE_TERRA_LCD_URL || 'http://localhost:1317',
      rpc: import.meta.env.VITE_TERRA_RPC_URL || 'http://localhost:26657',
    },
  },
  testnet: {
    terra: {
      chainId: 'rebel-2',
      lcd: 'https://terra-classic-lcd.publicnode.com',
      rpc: 'https://terra-classic-rpc.publicnode.com:443',
    },
  },
  mainnet: {
    terra: {
      chainId: 'columbus-5',
      lcd: 'https://terra-classic-lcd.publicnode.com',
      rpc: 'https://terra-classic-rpc.publicnode.com:443',
    },
  },
}

export const DEFAULT_NETWORK = (import.meta.env.VITE_NETWORK || 'local') as keyof typeof NETWORKS

/** Pair hard cap for batch cancel / claim execute msgs (`dex-common` `MAX_LIMIT_BATCH_RUNGS_HARD_CAP`). */
export const MAX_LIMIT_BATCH_RUNGS_HARD_CAP = 30

export function isValidTerraAddress(addr: string): boolean {
  return /^terra1[a-z0-9]{38,}$/.test(addr)
}
