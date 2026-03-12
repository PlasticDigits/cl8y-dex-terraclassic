import { create } from 'zustand'
import type { PairInfo } from '@/types'

interface DexState {
  selectedPair: PairInfo | null
  slippageTolerance: number
  setSelectedPair: (pair: PairInfo | null) => void
  setSlippageTolerance: (tolerance: number) => void
}

export const useDexStore = create<DexState>((set) => ({
  selectedPair: null,
  slippageTolerance: 0.5,
  setSelectedPair: (pair) => set({ selectedPair: pair }),
  setSlippageTolerance: (tolerance) => set({ slippageTolerance: tolerance }),
}))
