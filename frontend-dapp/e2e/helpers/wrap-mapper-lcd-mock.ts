import type { Page, Route } from '@playwright/test'

function decodeSmartQuery(url: string): { contract: string; query: Record<string, unknown> } | null {
  const match = url.match(/\/contract\/([^/]+)\/smart\/([^/?#]+)/)
  if (!match) return null
  const contract = match[1]
  const seg = decodeURIComponent(match[2])
  try {
    const query = JSON.parse(Buffer.from(seg, 'base64').toString('utf8')) as Record<string, unknown>
    return { contract, query }
  } catch {
    return null
  }
}

function lcdSmartResponse(body: unknown) {
  return {
    status: 200,
    contentType: 'application/json',
    body: JSON.stringify({ data: body }),
  }
}

/** Intercept wrap-mapper LCD smart queries for isolated SEC-A02 CTA tests (GitLab #389). */
export async function routeWrapMapperPaused(page: Page, wrapMapperAddress: string) {
  await page.route('**/*', async (route: Route) => {
    const url = route.request().url()
    if (!url.includes('/cosmwasm/wasm/v1/contract/')) {
      await route.continue()
      return
    }
    const decoded = decodeSmartQuery(url)
    if (!decoded || decoded.contract !== wrapMapperAddress) {
      await route.continue()
      return
    }
    if (decoded.query.config !== undefined) {
      await route.fulfill(
        lcdSmartResponse({
          governance: 'terra1gov',
          treasury: 'terra1treasury',
          paused: true,
          fee_bps: 0,
        })
      )
      return
    }
    await route.continue()
  })
}

/** Intercept wrap-mapper rate_limit query so any positive wrap amount exceeds the window cap. */
export async function routeWrapMapperRateLimitExceeded(page: Page, wrapMapperAddress: string) {
  await page.route('**/*', async (route: Route) => {
    const url = route.request().url()
    if (!url.includes('/cosmwasm/wasm/v1/contract/')) {
      await route.continue()
      return
    }
    const decoded = decodeSmartQuery(url)
    if (!decoded || decoded.contract !== wrapMapperAddress) {
      await route.continue()
      return
    }
    if (decoded.query.rate_limit !== undefined) {
      await route.fulfill(
        lcdSmartResponse({
          config: { max_amount_per_window: '1', window_seconds: 3600 },
          current_window_start: '1',
          amount_used: '0',
        })
      )
      return
    }
    if (decoded.query.config !== undefined) {
      await route.fulfill(
        lcdSmartResponse({
          governance: 'terra1gov',
          treasury: 'terra1treasury',
          paused: false,
          fee_bps: 0,
        })
      )
      return
    }
    await route.continue()
  })
}

export function wrapMapperAddressFromEnv(): string {
  const addr = process.env.VITE_WRAP_MAPPER_ADDRESS?.trim() ?? ''
  if (!addr.startsWith('terra1')) {
    throw new Error('VITE_WRAP_MAPPER_ADDRESS missing — run make deploy-local (writes frontend-dapp/.env.local)')
  }
  return addr
}
