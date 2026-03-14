import { Buffer } from 'buffer'
window.Buffer = Buffer

if (typeof crypto !== 'undefined' && !crypto.randomUUID) {
  crypto.randomUUID = () =>
    (([1e7] as unknown as string) + -1e3 + -4e3 + -8e3 + -1e11).replace(/[018]/g, (c: string) =>
      (Number(c) ^ (crypto.getRandomValues(new Uint8Array(1))[0] & (15 >> (Number(c) / 4)))).toString(16)
    ) as `${string}-${string}-${string}-${string}-${string}`
}

import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App.tsx'
import './index.css'
;(globalThis as Record<string, unknown>).Buffer = Buffer
;(globalThis as Record<string, unknown>).process = { env: {} }

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>
)
