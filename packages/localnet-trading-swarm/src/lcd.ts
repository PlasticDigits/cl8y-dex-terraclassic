const LCD_TIMEOUT_MS = 15_000

export function decodeWasmSmartData<T>(raw: unknown): T {
  if (raw === null || raw === undefined) {
    throw new Error('LCD returned empty wasm smart query data')
  }
  if (typeof raw === 'string') {
    const json = Buffer.from(raw, 'base64').toString('utf8')
    return JSON.parse(json) as T
  }
  return raw as T
}

export async function lcdFetchJson<T>(lcdBase: string, path: string): Promise<T> {
  const url = `${lcdBase.replace(/\/$/, '')}${path}`
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), LCD_TIMEOUT_MS)
  try {
    const res = await fetch(url, { signal: controller.signal })
    if (!res.ok) {
      let detail = `${res.status}`
      try {
        const err = (await res.json()) as { message?: string; error?: string }
        detail = err.message ?? err.error ?? detail
      } catch {
        /* ignore */
      }
      throw new Error(`LCD ${url}: ${detail}`)
    }
    return (await res.json()) as T
  } finally {
    clearTimeout(timer)
  }
}

export async function queryWasmSmart<T>(lcdBase: string, contract: string, queryMsg: object): Promise<T> {
  const q64 = Buffer.from(JSON.stringify(queryMsg), 'utf8').toString('base64')
  const path = `/cosmwasm/wasm/v1/contract/${contract}/smart/${q64}`
  const body = await lcdFetchJson<{ data: unknown }>(lcdBase, path)
  return decodeWasmSmartData<T>(body.data)
}
