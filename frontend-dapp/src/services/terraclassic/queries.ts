import { TERRA_LCD_URL } from '@/utils/constants'

export async function queryContract<T>(contractAddress: string, queryMsg: Record<string, unknown>): Promise<T> {
  const queryBase64 = btoa(JSON.stringify(queryMsg))
  const url = `${TERRA_LCD_URL}/terra/wasm/v1beta1/contracts/${contractAddress}/store?query_msg=${queryBase64}`
  const response = await fetch(url)
  if (!response.ok) {
    throw new Error(`Query failed: ${response.status}`)
  }
  const data = await response.json()
  return JSON.parse(atob(data.query_result)) as T
}
