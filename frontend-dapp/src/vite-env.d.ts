/// <reference types="vite/client" />

declare const __GIT_SHA__: string
declare const __APP_VERSION__: string

interface Window {
  Buffer: typeof Buffer
  station?: {
    connect: () => Promise<void>
    disconnect: () => Promise<void>
  }
  keplr?: {
    enable: (chainId: string) => Promise<void>
    getOfflineSigner: (chainId: string) => unknown
  }
}
