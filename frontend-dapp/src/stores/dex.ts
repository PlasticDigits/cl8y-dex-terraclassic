import { create } from 'zustand'
import type { PairInfo } from '@/types'

interface DexState {
  selectedPair: PairInfo | null
  slippageTolerance: number
  deadlineSeconds: number
  setSelectedPair: (pair: PairInfo | null) => void
  setSlippageTolerance: (tolerance: number) => void
  setDeadlineSeconds: (seconds: number) => void
}

export const useDexStore = create<DexState>((set) => ({
  selectedPair: null,
  slippageTolerance: 0.5,
  deadlineSeconds: 300,
  setSelectedPair: (pair) => set({ selectedPair: pair }),
  setSlippageTolerance: (tolerance) => set({ slippageTolerance: Math.min(50, Math.max(0.01, tolerance)) }),
  setDeadlineSeconds: (seconds) => set({ deadlineSeconds: Math.max(30, Math.min(3600, seconds)) }),
}))
