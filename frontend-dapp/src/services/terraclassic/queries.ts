import { TERRA_LCD_URL } from '@/utils/constants'

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
