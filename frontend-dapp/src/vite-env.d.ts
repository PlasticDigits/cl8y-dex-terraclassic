/// <reference types="vite/client" />

declare module '*.json' {
  const value: Record<string, unknown>
  export default value
}

interface ImportMetaEnv {
  /** When `"true"`, skips the blocking first-visit risk acknowledgement (Playwright webServer only — GitLab #138). */
  readonly VITE_PLAYWRIGHT_E2E?: string
  /** Soft-launch faucet contract (GitLab #473). */
  readonly VITE_FAUCET_ADDRESS?: string
  readonly VITE_TOKEN_EMBER_ADDRESS?: string
  readonly VITE_TOKEN_CORAL_ADDRESS?: string
  readonly VITE_TOKEN_JADE_ADDRESS?: string
  readonly VITE_TOKEN_ONYX_ADDRESS?: string
  readonly VITE_TOKEN_RUBY_ADDRESS?: string
  readonly VITE_TOKEN_TOPAZ_ADDRESS?: string
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
