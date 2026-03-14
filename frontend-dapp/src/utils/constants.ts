export const TERRA_LCD_URL = import.meta.env.VITE_TERRA_LCD_URL || 'https://terra-classic-lcd.publicnode.com'
export const TERRA_RPC_URL = import.meta.env.VITE_TERRA_RPC_URL || 'https://terra-classic-rpc.publicnode.com:443'
export const FACTORY_CONTRACT_ADDRESS = import.meta.env.VITE_FACTORY_ADDRESS || ''
export const ROUTER_CONTRACT_ADDRESS = import.meta.env.VITE_ROUTER_ADDRESS || ''
export const FEE_DISCOUNT_CONTRACT_ADDRESS = import.meta.env.VITE_FEE_DISCOUNT_ADDRESS || ''
export const CL8Y_TOKEN_ADDRESS =
  import.meta.env.VITE_CL8Y_TOKEN_ADDRESS || 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'

export const DEV_MODE = import.meta.env.VITE_DEV_MODE === 'true'

export const GAS_PRICE_ULUNA = import.meta.env.VITE_GAS_PRICE_ULUNA || '28.325'

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
