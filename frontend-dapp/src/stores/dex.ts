import { create } from 'zustand'
import type { PairInfo, ReservesInfo } from '@/types'

interface DexState {
  pairs: PairInfo[]
  selectedPair: PairInfo | null
  reserves: ReservesInfo | null
  slippageTolerance: number
  setPairs: (pairs: PairInfo[]) => void
  setSelectedPair: (pair: PairInfo | null) => void
  setReserves: (reserves: ReservesInfo | null) => void
  setSlippageTolerance: (tolerance: number) => void
}

export const useDexStore = create<DexState>((set) => ({
  pairs: [],
  selectedPair: null,
  reserves: null,
  slippageTolerance: 0.5,
  setPairs: (pairs) => set({ pairs }),
  setSelectedPair: (pair) => set({ selectedPair: pair }),
  setReserves: (reserves) => set({ reserves }),
  setSlippageTolerance: (tolerance) => set({ slippageTolerance: tolerance }),
}))
