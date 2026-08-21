import { getChainContractInfo } from './queries'
import { getAssetCodeIds, getPairInfo } from './pair'
import { isCodeIdWhitelisted } from './factory'
import { evaluateLivePins, isPreF6AssetCodeIdsError, type CodeIdFreezeVerdict } from '@/utils/assetCodeIdFreeze'
import type { AssetInfo } from '@/types'

export interface PairCodeIdFreezeProbe {
  frozen: boolean
  verdict: CodeIdFreezeVerdict | 'unknown'
}

function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err)
}

function cw20FromPairInfo(info: AssetInfo): string | null {
  if ('token' in info && info.token?.contract_addr?.startsWith('terra1')) {
    return info.token.contract_addr
  }
  return null
}

/**
 * LCD F6 probe: `GetAssetCodeIds` vs live `ContractInfo.code_id` + factory whitelist.
 * Pre-1.15.0 / unpinned → not frozen. Transport errors → unknown (caller fail-opens).
 */
export async function probePairCodeIdFreeze(pairAddress: string): Promise<PairCodeIdFreezeProbe> {
  let pins: [number, number]
  try {
    const resp = await getAssetCodeIds(pairAddress)
    const ids = resp.code_ids
    if (!Array.isArray(ids) || ids.length !== 2) {
      return { frozen: false, verdict: 'unknown' }
    }
    pins = [Number(ids[0]), Number(ids[1])]
  } catch (err) {
    if (isPreF6AssetCodeIdsError(errorMessage(err))) {
      return { frozen: false, verdict: 'tradable' }
    }
    return { frozen: false, verdict: 'unknown' }
  }

  let t0: string | null
  let t1: string | null
  try {
    const pair = await getPairInfo(pairAddress)
    t0 = cw20FromPairInfo(pair.asset_infos[0])
    t1 = cw20FromPairInfo(pair.asset_infos[1])
  } catch {
    return { frozen: false, verdict: 'unknown' }
  }
  if (!t0 || !t1) {
    return { frozen: false, verdict: 'tradable' }
  }

  let live0: number
  let live1: number
  try {
    live0 = (await getChainContractInfo(t0)).code_id
    live1 = (await getChainContractInfo(t1)).code_id
  } catch {
    return { frozen: false, verdict: 'unknown' }
  }
  if (!Number.isFinite(live0) || !Number.isFinite(live1)) {
    return { frozen: false, verdict: 'unknown' }
  }

  let wl0: boolean
  let wl1: boolean
  try {
    wl0 = (await isCodeIdWhitelisted(live0)).whitelisted
    wl1 = (await isCodeIdWhitelisted(live1)).whitelisted
  } catch {
    return { frozen: false, verdict: 'unknown' }
  }

  const verdict = evaluateLivePins({
    pin0: pins[0],
    pin1: pins[1],
    live0,
    live1,
    whitelisted0: wl0,
    whitelisted1: wl1,
  })
  return { frozen: verdict === 'frozen', verdict }
}
