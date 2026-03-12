import type { AssetInfo } from '@/types'
import { TERRA_LCD_URL } from '@/utils/constants'

export async function getChainContractInfo(contractAddress: string): Promise<{ code_id: number; creator: string; admin: string; label: string }> {
  const url = `${TERRA_LCD_URL}/cosmwasm/wasm/v1/contract/${contractAddress}`
  const response = await fetch(url)
  if (!response.ok) {
    throw new Error(`Failed to fetch contract info: ${response.status}`)
  }
  const data = await response.json()
  const info = data.contract_info
  return {
    code_id: Number(info.code_id),
    creator: info.creator,
    admin: info.admin,
    label: info.label,
  }
}

export async function queryContract<T>(contractAddress: string, queryMsg: Record<string, unknown>): Promise<T> {
  const queryBase64 = btoa(JSON.stringify(queryMsg))
  const url = `${TERRA_LCD_URL}/cosmwasm/wasm/v1/contract/${contractAddress}/smart/${queryBase64}`
  const response = await fetch(url)
  if (!response.ok) {
    let errorDetail = `Query failed: ${response.status}`
    try {
      const errorData = await response.json()
      if (errorData?.message) {
        errorDetail = errorData.message
      } else if (errorData?.error) {
        errorDetail = typeof errorData.error === 'string' ? errorData.error : JSON.stringify(errorData.error)
      }
    } catch {
      // use default error detail
    }
    throw new Error(errorDetail)
  }
  const data = await response.json()
  return data.data as T
}

export async function getTokenBalance(walletAddress: string, assetInfo: AssetInfo): Promise<string> {
  if ('token' in assetInfo) {
    const resp = await queryContract<{ balance: string }>(assetInfo.token.contract_addr, {
      balance: { address: walletAddress },
    })
    return resp.balance
  }
  const denom = assetInfo.native_token.denom
  const url = `${TERRA_LCD_URL}/cosmos/bank/v1beta1/balances/${walletAddress}/by_denom?denom=${denom}`
  const response = await fetch(url)
  if (!response.ok) throw new Error(`Balance query failed: ${response.status}`)
  const data = await response.json()
  return data.balance?.amount ?? '0'
}
