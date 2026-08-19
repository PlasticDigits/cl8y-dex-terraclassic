/// <reference types="vite/client" />

declare module '*.json' {
  const value: Record<string, unknown>
  export default value
}

interface ImportMetaEnv {
  /** When `"true"`, skips the blocking first-visit risk acknowledgement and Legal clickwrap gate (Playwright webServer only — GitLab #138 / #517). */
  readonly VITE_PLAYWRIGHT_E2E?: string
  /** Legal API base (default https://api.terms.cl8y.com) — GitLab #517. */
  readonly VITE_LEGAL_API_BASE_URL?: string
  /** Legal portal base (default https://terms.cl8y.com) — GitLab #517. */
  readonly VITE_LEGAL_TERMS_BASE_URL?: string
  /** Legal property hostname (default dex.cl8y.com) — GitLab #517. */
  readonly VITE_LEGAL_PROPERTY?: string
  /** Optional extra comma-separated redirect origins for portal preflight — GitLab #517. */
  readonly VITE_LEGAL_REDIRECT_ALLOWLIST?: string
  /** Soft-launch faucet contract (GitLab #473). */
  readonly VITE_FAUCET_ADDRESS?: string
  readonly VITE_TOKEN_EMBER_ADDRESS?: string
  readonly VITE_TOKEN_CORAL_ADDRESS?: string
  readonly VITE_TOKEN_JADE_ADDRESS?: string
  readonly VITE_TOKEN_ONYX_ADDRESS?: string
  readonly VITE_TOKEN_RUBY_ADDRESS?: string
  readonly VITE_TOKEN_TOPAZ_ADDRESS?: string
  /** QA-only: show gem browse on a `VITE_NETWORK=mainnet` build (GitLab #562). Never a query param. */
  readonly VITE_SHOW_TEST_TOKENS?: string
  /** UST1 oracle window (GitLab #506) — columbus-5 addresses on prod Coolify. */
  readonly VITE_UST1_WINDOW_ADDRESS?: string
  readonly VITE_UST1_TOKEN_ADDRESS?: string
  readonly VITE_VFDUSD_TOKEN_ADDRESS?: string
  readonly VITE_UST1_ORACLE_ADDRESS?: string
}

declare module 'react-blockies'

declare const __GIT_SHA__: string
declare const __APP_VERSION__: string

interface Window {
  Buffer: typeof Buffer
  station?: {
    connect: () => Promise<void>
    disconnect: () => Promise<void>
    /** Native Station network registry (required for LocalTerra on new Station — GitLab #207). */
    addNetwork?: (network: {
      name: string
      chainID: string
      lcd: string
      prefix?: string
      coinType?: string
      baseAsset?: string
      gasAdjustment?: number
      gasPrices?: Record<string, number>
    }) => Promise<boolean>
    hasNetwork?: (network: { chainID: string; lcd: string }) => Promise<boolean>
    /** Keplr-compatible API; supports `experimentalSuggestChain` for LocalTerra gas steps (GitLab #127). */
    keplr?: {
      enable: (chainIds: string | string[]) => Promise<void>
      experimentalSuggestChain?: (chainInfo: Record<string, unknown>) => Promise<void>
      getKey: (chainId: string) => Promise<{
        name: string
        bech32Address: string
        pubKey: Uint8Array
        isNanoLedger: boolean
      }>
      getOfflineSigner: (chainId: string) => unknown
      defaultOptions?: { sign?: { preferNoSetFee?: boolean; preferNoSetMemo?: boolean } }
      signAmino?: (...args: unknown[]) => Promise<unknown>
      signDirect?: (...args: unknown[]) => Promise<unknown>
    }
  }
  keplr?: {
    enable: (chainId: string) => Promise<void>
    experimentalSuggestChain: (chainInfo: Record<string, unknown>) => Promise<void>
    getOfflineSigner: (chainId: string) => unknown
  }
}
