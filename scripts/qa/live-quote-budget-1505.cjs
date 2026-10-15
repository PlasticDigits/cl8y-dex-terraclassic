// Observer resource limits only; application polling and measurement clocks are unchanged.
const CONTEXT_QUOTE_MAXIMUM = 600;
const GENERIC = Object.freeze({
  measurementVersion: "foreground_correlated_native_availability_v8",
  quoteProgressBudget: 600,
  shardCount: 3,
  cohortQuoteProgressBudget: 1800,
});
const NATIVE = Object.freeze({
  measurementVersion: "foreground_correlated_native_availability_v9",
  // Includes four initialized API solves and every browser setup/solve/progress read.
  // Observed full windows used 152/162/151 reads; 800 retains finite headroom
  // for four complete 190-second windows without changing application cadence.
  quoteProgressBudget: 800,
  shardCount: 1,
  cohortQuoteProgressBudget: 800,
});

function quoteBudgetProfile(native1505) {
  return native1505 ? NATIVE : GENERIC;
}

function contextQuoteAllowance(totalBudget, consumed) {
  if (
    !Number.isSafeInteger(totalBudget) ||
    totalBudget < 1 ||
    totalBudget > NATIVE.quoteProgressBudget ||
    !Number.isSafeInteger(consumed) ||
    consumed < 0
  )
    throw Error("invalid_quote_budget_accounting");
  // The shared read guard keeps its original 600 maximum. This does not reset
  // the caller's global counter or grant each context a new cohort budget.
  return Math.min(CONTEXT_QUOTE_MAXIMUM, Math.max(0, totalBudget - consumed));
}

function quoteBudgetExhausted(totalBudget, consumed, contextStart, contextAllowance) {
  return consumed >= totalBudget || consumed - contextStart >= contextAllowance;
}

module.exports = {
  CONTEXT_QUOTE_MAXIMUM,
  quoteBudgetProfile,
  contextQuoteAllowance,
  quoteBudgetExhausted,
};
