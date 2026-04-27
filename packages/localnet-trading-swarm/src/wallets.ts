import * as bip39 from '@scure/bip39'
import { wordlist } from '@scure/bip39/wordlists/english'
import { MnemonicWallet } from '@goblinhunt/cosmes/wallet'

export interface SwarmWallet {
  profileId: string
  index: number
  wallet: MnemonicWallet
  address: string
}

export function resolveSwarmMnemonic(): string {
  const fromEnv = process.env.SWARM_BOT_MNEMONIC?.trim()
  if (fromEnv) {
    if (!bip39.validateMnemonic(fromEnv, wordlist)) {
      throw new Error('SWARM_BOT_MNEMONIC is set but is not a valid BIP39 English mnemonic.')
    }
    return fromEnv
  }
  return bip39.generateMnemonic(wordlist, 128)
}

export function createSwarmWallets(opts: {
  mnemonic: string
  chainId: string
  rpc: string
  gasPriceUluna: string
  count: number
}): MnemonicWallet[] {
  const out: MnemonicWallet[] = []
  for (let i = 0; i < opts.count; i++) {
    out.push(
      new MnemonicWallet({
        mnemonic: opts.mnemonic,
        bech32Prefix: 'terra',
        chainId: opts.chainId,
        rpc: opts.rpc,
        gasPrice: { amount: opts.gasPriceUluna, denom: 'uluna' },
        coinType: 330,
        index: i,
      })
    )
  }
  return out
}
