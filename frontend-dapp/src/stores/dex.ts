import { create } from 'zustand'
import type { PairInfo, PoolResponse } from '@/types'

interface DexState {
  pairs: PairInfo[]
  selectedPair: PairInfo | null
  pool: PoolResponse | null
  slippageTolerance: number
  setPairs: (pairs: PairInfo[]) => void
  setSelectedPair: (pair: PairInfo | null) => void
  setPool: (pool: PoolResponse | null) => void
  setSlippageTolerance: (tolerance: number) => void
}

export const useDexStore = create<DexState>((set) => ({
  pairs: [],
  selectedPair: null,
  pool: null,
  slippageTolerance: 0.5,
  setPairs: (pairs) => set({ pairs }),
  setSelectedPair: (pair) => set({ selectedPair: pair }),
  setPool: (pool) => set({ pool }),
  setSlippageTolerance: (tolerance) => set({ slippageTolerance: tolerance }),
}))
