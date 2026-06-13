import {
  FACTORY_CONTRACT_ADDRESS,
  ROUTER_CONTRACT_ADDRESS,
  TERRA_LCD_URL,
  isValidTerraAddress,
} from '@/utils/constants'

export interface DeployAddressVerificationResult {
  ok: boolean
  configuredFactory: string
  configuredRouter: string
  lcdFactory?: string
  error?: string
}

/**
 * Optional startup check: query factory `config` on LCD and compare `factory` field wiring.
 * Enable with `VITE_VERIFY_DEPLOY_ADDRESSES=true` at build time (GitLab #378).
 */
export async function verifyDeployAddressesOnLcd(): Promise<DeployAddressVerificationResult> {
  const configuredFactory = FACTORY_CONTRACT_ADDRESS.trim()
  const configuredRouter = ROUTER_CONTRACT_ADDRESS.trim()

  if (!configuredFactory || !isValidTerraAddress(configuredFactory)) {
    return {
      ok: false,
      configuredFactory,
      configuredRouter,
      error: 'VITE_FACTORY_ADDRESS is missing or invalid',
    }
  }

  if (!configuredRouter || !isValidTerraAddress(configuredRouter)) {
    return {
      ok: false,
      configuredFactory,
      configuredRouter,
      error: 'VITE_ROUTER_ADDRESS is missing or invalid',
    }
  }

  try {
    const url = `${TERRA_LCD_URL.replace(/\/$/, '')}/cosmwasm/wasm/v1/contract/${configuredRouter}/smart/${btoa(
      JSON.stringify({ config: {} })
    )}`
    const res = await fetch(url)
    if (!res.ok) {
      return {
        ok: false,
        configuredFactory,
        configuredRouter,
        error: `LCD router config query failed (${res.status})`,
      }
    }
    const body = (await res.json()) as { data?: { factory?: string } }
    const lcdFactory = body.data?.factory?.trim()
    if (!lcdFactory) {
      return {
        ok: false,
        configuredFactory,
        configuredRouter,
        lcdFactory,
        error: 'Router config response missing factory address',
      }
    }
    if (lcdFactory !== configuredFactory) {
      return {
        ok: false,
        configuredFactory,
        configuredRouter,
        lcdFactory,
        error: 'VITE_FACTORY_ADDRESS does not match router on-chain factory',
      }
    }
    return { ok: true, configuredFactory, configuredRouter, lcdFactory }
  } catch (err) {
    return {
      ok: false,
      configuredFactory,
      configuredRouter,
      error: err instanceof Error ? err.message : 'LCD verification failed',
    }
  }
}
