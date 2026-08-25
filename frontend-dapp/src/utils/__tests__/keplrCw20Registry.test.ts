import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { COLUMBUS5_GEM_ADDRESSES } from '../pairCatalogRank'
import { lookupByCW20 } from '../tokenRegistry'
import {
  buildKeplrCw20TokenJson,
  CL8Y_COINGECKO_ID,
  isKeplrRecognitionCw20,
  KEPLR_CW20_CATALOG,
  KEPLR_CW20_CHAIN_DIR,
  keplrCw20ForbiddenPriceKeys,
  keplrCw20TokenFilename,
  keplrCw20TokensToSubmit,
  lookupKeplrCw20,
} from '../keplrCw20Registry'

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../../..')
const PACK_TOKENS = resolve(REPO_ROOT, 'docs/listings/keplr-contract-registry/cosmos/columbus/tokens')

const THIRD_PARTY_NOT_IN_PACK = [
  'terra1x6e64es6yhauhvs3prvpdg2gkqdtfru840wgnhs935x8axr7zxkqzysuxz', // ALPHA
  'terra1r3eaa2tucjr3es88wzuqpgxvssqflk9cghrjmf9uneds8wljyapqwtrcp5', // USTRIX
  'terra1cvd5cgrs8rrl96hte34n57497u5f9cwuv3e6ztxgetkx4uzmcdyswv79zl', // SpaceUSD
]

describe('keplrCw20Registry (GitLab #629)', () => {
  it('K629-1 uses columbus, not terra or phoenix', () => {
    expect(KEPLR_CW20_CHAIN_DIR).toBe('columbus')
  })

  it('K629-2 lists the six permanent CW20s and no gems or third-party extras', () => {
    expect(KEPLR_CW20_CATALOG.map((t) => t.symbol)).toEqual(['CL8Y', 'UST1', 'USTR', 'cLUNC', 'cUSTC', 'vFDUSD'])
    for (const entry of KEPLR_CW20_CATALOG) {
      expect(COLUMBUS5_GEM_ADDRESSES.has(entry.contractAddress)).toBe(false)
    }
    for (const addr of THIRD_PARTY_NOT_IN_PACK) {
      expect(isKeplrRecognitionCw20(addr)).toBe(false)
    }
    expect(keplrCw20TokensToSubmit()).toHaveLength(5)
  })

  it('K629-3 pins on-chain decimals (CL8Y/USTR 18, others 6)', () => {
    expect(lookupKeplrCw20('terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3')?.decimals).toBe(18)
    expect(lookupKeplrCw20('terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv')?.decimals).toBe(18)
    for (const symbol of ['UST1', 'cLUNC', 'cUSTC', 'vFDUSD'] as const) {
      expect(KEPLR_CW20_CATALOG.find((t) => t.symbol === symbol)?.decimals).toBe(6)
    }
    expect(lookupByCW20('terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3')?.decimals).toBe(18)
  })

  it('K629-4 only CL8Y may carry coinGeckoId; no price/oracle fields', () => {
    for (const entry of KEPLR_CW20_CATALOG) {
      const json = buildKeplrCw20TokenJson(entry)
      for (const key of keplrCw20ForbiddenPriceKeys()) {
        expect(key in json).toBe(false)
      }
      if (entry.symbol === 'CL8Y') {
        expect(json.coinGeckoId).toBe(CL8Y_COINGECKO_ID)
      } else {
        expect(json.coinGeckoId).toBeUndefined()
      }
    }
  })

  it('K629-6 confirms vFDUSD address and 6 decimals', () => {
    const vfd = lookupKeplrCw20('terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3')
    expect(vfd?.symbol).toBe('vFDUSD')
    expect(vfd?.decimals).toBe(6)
    expect(vfd?.status).toBe('submit')
  })

  it('K629-8 marks USTR already_registered as USTC Repeg', () => {
    const ustr = KEPLR_CW20_CATALOG.find((t) => t.symbol === 'USTR')
    expect(ustr?.status).toBe('already_registered')
    expect(ustr?.name).toBe('USTC Repeg')
    expect(ustr?.decimals).toBe(18)
  })

  it('submission JSON files match the catalog builder', () => {
    for (const entry of KEPLR_CW20_CATALOG) {
      const path = resolve(PACK_TOKENS, keplrCw20TokenFilename(entry.contractAddress))
      const onDisk = JSON.parse(readFileSync(path, 'utf8')) as Record<string, unknown>
      expect(onDisk).toEqual(buildKeplrCw20TokenJson(entry))
    }
  })
})
