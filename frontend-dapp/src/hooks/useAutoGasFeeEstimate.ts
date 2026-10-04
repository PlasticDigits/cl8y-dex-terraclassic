import { useEffect, useState } from 'react'
import type { TerraClassicFeeEstimate } from '@/services/terraclassic/terraClassicFeeEstimate'
import {
  feeEstimateForGasLimit,
  resolveSwapAutoGas,
  type SwapGasReadRequest,
} from '@/services/terraclassic/swapAutoGas'

function probeKey(probe: SwapGasReadRequest | null): string {
  if (!probe) return ''
  return JSON.stringify({ signer: probe.signer, entries: probe.entries })
}

/**
 * Static fallback until simulate resolves. A failed first simulate climbs in 200k
 * steps before the signature so the Network fee matches the signed gas (#1360).
 */
export function useAutoGasFeeEstimate(
  fallback: TerraClassicFeeEstimate,
  probe: SwapGasReadRequest | null
): TerraClassicFeeEstimate {
  const key = probeKey(probe)
  const [resolved, setResolved] = useState<TerraClassicFeeEstimate | null>(null)
  const [resolvedKey, setResolvedKey] = useState('')

  useEffect(() => {
    if (!probe) return
    let cancelled = false
    void resolveSwapAutoGas(fallback.gasLimit, probe).then((decision) => {
      if (cancelled) return
      setResolvedKey(key)
      setResolved(decision.source === 'simulate' ? feeEstimateForGasLimit(decision.gasLimit) : null)
    })
    return () => {
      cancelled = true
    }
    // fallback.gasLimit is the static envelope for this probe; key covers the messages.
    // eslint-disable-next-line react-hooks/exhaustive-deps -- probe identity is `key`
  }, [key, fallback.gasLimit])

  if (!probe || resolvedKey !== key) return fallback
  return resolved ?? fallback
}
