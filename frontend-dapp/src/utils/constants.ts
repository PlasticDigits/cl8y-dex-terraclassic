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

export const GAS_PRICE_ULUNA = import.meta.env.VITE_GAS_PRICE_ULUNA || '28.325'
export const SWAP_GAS_PER_HOP = 600000
/** Multiplier on (per-hop base × hop count) before floor/padding. */
export const SWAP_GAS_BUFFER = 1.1
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

const EXPLORER_TX_URLS: Record<string, string> = {
  local: 'http://localhost:1317/cosmos/tx/v1beta1/txs/',
  testnet: 'https://finder.terra-classic.hexxagon.io/testnet/tx/',
  mainnet: 'https://finder.terra-classic.hexxagon.io/mainnet/tx/',
}

export function getExplorerTxUrl(txHash: string): string | null {
  const base = EXPLORER_TX_URLS[DEFAULT_NETWORK]
  if (!base) return null
  return `${base}${txHash}`
}

export function isValidTerraAddress(addr: string): boolean {
  return /^terra1[a-z0-9]{38,}$/.test(addr)
}
