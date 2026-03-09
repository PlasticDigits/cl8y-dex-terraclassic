export interface PairInfo {
  token_a: string
  token_b: string
  pair_contract: string
  lp_token: string
}

export interface FeeConfig {
  fee_bps: number
  treasury: string
}

export interface ReservesInfo {
  reserve_a: string
  reserve_b: string
}

export interface SimulateSwapResult {
  return_amount: string
  fee_amount: string
  spread_amount: string
}
