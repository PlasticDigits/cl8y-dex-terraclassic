/** Debounce before swap / market sim queries fire (GitLab #346). */
export const SIM_QUOTE_DEBOUNCE_MS = 350

/** Block submit while typed pay size differs from the debounced sim key or prior quote is still shown. */
export function isSimQuoteStaleForSubmit(
  rawInputAmount: string,
  debouncedRawInputAmount: string,
  isPlaceholderData: boolean
): boolean {
  return rawInputAmount !== debouncedRawInputAmount || isPlaceholderData
}
