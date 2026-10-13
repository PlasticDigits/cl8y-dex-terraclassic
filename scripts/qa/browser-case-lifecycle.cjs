// Resource cleanup belongs after the complete measured observation. This helper
// cannot alter quote clocks, phase deadlines, response data or signing behavior.
async function recordThenClearInput(pay, record) {
  const sample = await record();
  await pay.fill("0");
  return sample;
}

function preserveFailureClass(first, later) {
  return first ?? later;
}

// Attempts beyond the cap are still recorded, but cannot reach the network.
function countBoundedRead(current, maximum) {
  const attempted = current + 1;
  return { attempted, allowed: attempted <= maximum };
}

function requestFamily(pathname) {
  if (pathname === "/api/v1/route/solve/progress") return "route_progress";
  if (pathname === "/api/v1/route/solve/best") return "route_best";
  if (pathname === "/api/v1/route/solve") return "route_quote";
  if (pathname.startsWith("/api/v1/route/")) return "route_other";
  if (pathname.startsWith("/api/v1/pairs")) return "pair_data";
  if (pathname.startsWith("/api/v1/tokens")) return "token_data";
  return "other_public";
}

function recordRequestDiagnostic(families, pathname, allowed) {
  const family = requestFamily(pathname);
  const counters = (families[family] ??= { attempted: 0, budget_admitted: 0 });
  counters.attempted++;
  if (allowed) counters.budget_admitted++;
}

module.exports = {
  recordThenClearInput,
  preserveFailureClass,
  countBoundedRead,
  requestFamily,
  recordRequestDiagnostic,
};
