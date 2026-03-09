import { MnemonicWallet } from '@goblinhunt/cosmes/wallet'
import { NETWORKS, DEFAULT_NETWORK } from '@/utils/constants'

const DEV_MNEMONIC = 'notice oak worry limit wrap speak medal online prefer cluster roof addict wrist behave treat actual wasp year salad speed social layer crew genius'

export const DEV_TERRA_ADDRESS = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'

const GAS_PRICE = {
  amount: '28.325',
  denom: 'uluna',
}

export function createDevTerraWallet(): MnemonicWallet {
  const networkConfig = NETWORKS[DEFAULT_NETWORK].terra
  return new MnemonicWallet({
    mnemonic: DEV_MNEMONIC,
    bech32Prefix: 'terra',
    chainId: networkConfig.chainId,
    rpc: networkConfig.rpc,
    gasPrice: GAS_PRICE,
    coinType: 330,
    index: 0,
  })
}
