/** LCD `token_info` mock for Swap / Trade amount-scale tests (Forgejo #1255). */
export function lcdTokenInfoResponse(decimals: unknown = 6) {
  return { name: 'Dummy', symbol: 'DUM', decimals, total_supply: '0' }
}

export function queryContractTokenInfoImpl(decimals: unknown = 6) {
  return async (_addr: string, msg: unknown) => {
    if (msg && typeof msg === 'object' && 'token_info' in msg) {
      return lcdTokenInfoResponse(decimals)
    }
    return {}
  }
}
