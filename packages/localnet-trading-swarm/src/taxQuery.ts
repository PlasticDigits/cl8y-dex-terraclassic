import { queryWasmSmart } from './lcd.js'
import type { TaxPreviewView } from './taxPreview.js'
import { DEFAULT_SELL_BPS, normalizeTaxTokens, taxTokenFromEnv } from './taxDetect.js'

export async function queryCw20Balance(lcdBase: string, token: string, holder: string): Promise<bigint> {
  try {
    const raw = await queryWasmSmart<{ balance: string }>(lcdBase, token, {
      balance: { address: holder },
    })
    const b = raw.balance ?? '0'
    return /^\d+$/.test(b) ? BigInt(b) : 0n
  } catch {
    return 0n
  }
}

export async function querySellBps(lcdBase: string, taxToken: string): Promise<number> {
  try {
    const cfg = await queryWasmSmart<{ sell_bps?: number }>(lcdBase, taxToken, { get_config: {} })
    const bps = Number(cfg.sell_bps)
    return Number.isFinite(bps) && bps >= 0 ? bps : DEFAULT_SELL_BPS
  } catch {
    return DEFAULT_SELL_BPS
  }
}

export async function queryTaxPreview(input: {
  lcdBase: string
  token: string
  from: string
  to: string
  amount: string
  sendMsgB64?: string | null
}): Promise<TaxPreviewView | null> {
  try {
    return await queryWasmSmart<TaxPreviewView>(input.lcdBase, input.token, {
      tax_preview: {
        from: input.from,
        to: input.to,
        amount: input.amount,
        send_msg: input.sendMsgB64 ?? null,
      },
    })
  } catch {
    return null
  }
}

export async function discoverTaxTokens(
  lcdBase: string,
  cw20Tokens: string[],
  env: Record<string, string | undefined>
): Promise<Set<string>> {
  const pinned = taxTokenFromEnv(env)
  const found: string[] = pinned ? [pinned] : []
  for (const token of cw20Tokens) {
    if (token === pinned) continue
    try {
      const origin = await queryWasmSmart<{ launcher?: string | null }>(lcdBase, token, {
        get_launcher_origin: {},
      })
      if (origin.launcher && origin.launcher.startsWith('terra1')) found.push(token)
    } catch {
      /* gem / wrap — no GetLauncherOrigin */
    }
  }
  return normalizeTaxTokens(found)
}
