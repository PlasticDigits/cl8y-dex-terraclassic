/// <reference types="vite/client" />

declare module '*.json' {
  const value: Record<string, unknown>
  export default value
}

declare module 'react-blockies'

declare const __GIT_SHA__: string
declare const __APP_VERSION__: string

interface Window {
  Buffer: typeof Buffer
  station?: {
    connect: () => Promise<void>
    disconnect: () => Promise<void>
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
