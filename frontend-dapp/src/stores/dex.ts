import { create } from 'zustand'
import type { PairInfo } from '@/types'
import { readExpertMode, writeExpertMode } from '@/utils/expertMode'
import { DEFAULT_SLIPPAGE_TOLERANCE_PERCENT } from '@/utils/slippageProtectionCopy'

interface DexState {
  selectedPair: PairInfo | null
  /** Percent; default {@link DEFAULT_SLIPPAGE_TOLERANCE_PERCENT} (GitLab #497). */
  slippageTolerance: number
  deadlineSeconds: number
  /** Off by default — allows swaps with &gt;30% route slippage when enabled (GitLab #293). */
  expertMode: boolean
  setSelectedPair: (pair: PairInfo | null) => void
  setSlippageTolerance: (tolerance: number) => void
  setDeadlineSeconds: (seconds: number) => void
  setExpertMode: (enabled: boolean) => void
}

export const useDexStore = create<DexState>((set) => ({
  selectedPair: null,
  slippageTolerance: DEFAULT_SLIPPAGE_TOLERANCE_PERCENT,
  deadlineSeconds: 300,
  expertMode: readExpertMode(),
  setSelectedPair: (pair) => set({ selectedPair: pair }),
  setSlippageTolerance: (tolerance) => set({ slippageTolerance: Math.min(50, Math.max(0.01, tolerance)) }),
  setDeadlineSeconds: (seconds) => set({ deadlineSeconds: Math.max(30, Math.min(3600, seconds)) }),
  setExpertMode: (enabled) => {
    writeExpertMode(enabled)
    set({ expertMode: enabled })
  },
}))
