import { MnemonicWallet } from '@goblinhunt/cosmes/wallet'
import { NETWORKS, DEFAULT_NETWORK, DEV_MODE } from '@/utils/constants'
import { registerConnectedWallet } from './wallet'

const GAS_PRICE = {
  amount: '28.325',
  denom: 'uluna',
}

function requireDevMnemonic(): string {
  const m = import.meta.env.VITE_DEV_MNEMONIC
  if (typeof m === 'string' && m.trim().length > 0) {
    return m.trim()
  }
  throw new Error(
    'VITE_DEV_MNEMONIC is required for the dev wallet. Add it to .env.development or .env.local (see .env.example). ' +
      'For LocalTerra, use the same value as TEST_MNEMONIC in docker/init-chain.sh (GitLab #118).'
  )
}

export function createDevTerraWallet(): MnemonicWallet {
  if (!DEV_MODE) {
    throw new Error('Dev wallet is only available in dev mode (VITE_DEV_MODE=true)')
  }

  const mnemonic = requireDevMnemonic()
  const networkConfig = NETWORKS[DEFAULT_NETWORK].terra
  const wallet = new MnemonicWallet({
    mnemonic,
    bech32Prefix: 'terra',
    chainId: networkConfig.chainId,
    rpc: networkConfig.rpc,
    gasPrice: GAS_PRICE,
    coinType: 330,
    index: 0,
  })
  registerConnectedWallet(wallet)
  return wallet
}
