export const TERRA_LCD_URL = import.meta.env.VITE_TERRA_LCD_URL || 'https://terra-classic-lcd.publicnode.com'
export const TERRA_RPC_URL = import.meta.env.VITE_TERRA_RPC_URL || 'https://terra-classic-rpc.publicnode.com:443'
export const FACTORY_CONTRACT_ADDRESS = import.meta.env.VITE_FACTORY_ADDRESS || ''
export const ROUTER_CONTRACT_ADDRESS = import.meta.env.VITE_ROUTER_ADDRESS || ''

export const DEV_MODE = import.meta.env.VITE_DEV_MODE !== 'false' && import.meta.env.MODE !== 'production'

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
      lcd: 'http://localhost:1317',
      rpc: 'http://localhost:26657',
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
