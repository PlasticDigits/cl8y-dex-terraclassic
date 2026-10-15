// In-page foreground clock; use with page.evaluate before the original input event.
// All useful timestamps originate in the browser, never a later Node polling roundtrip.
function installInputClock() {
  const pay = document.querySelector('[data-testid="swap-you-pay-amount"]');
  const receive = document.querySelector('[data-testid="swap-you-receive"]');
  if (!pay || !receive) throw new Error("clock_fields_unavailable");
  const clock = (window.__liveClock1499 = {
    started: null,
    started_epoch_ms: null,
    candidates: [],
    dom_observations: [],
    long_tasks: [],
    frame_gaps: [],
    long_task_observer_available: false,
    stopped: false,
  });
  let timer,
    frames = 0,
    lastValue = null,
    lastFrame = null,
    observer;
  if (typeof PerformanceObserver === "function") {
    try {
      observer = new PerformanceObserver((list) => {
        for (const entry of list.getEntries())
          if (
            clock.started != null &&
            entry.startTime >= clock.started &&
            clock.long_tasks.length < 32
          )
            clock.long_tasks.push({
              at_ms: entry.startTime - clock.started,
              duration_ms: entry.duration,
            });
      });
      observer.observe({ type: "longtask", buffered: true });
      clock.long_task_observer_available = true;
    } catch {
      /* diagnostic unavailable; useful clock is unchanged */
    }
  }
  const visible = (node) => {
    const r = node.getBoundingClientRect(),
      s = getComputedStyle(node);
    return (
      document.visibilityState === "visible" &&
      !document.hidden &&
      r.width > 0 &&
      r.height > 0 &&
      r.bottom > 0 &&
      r.top < innerHeight &&
      r.right > 0 &&
      r.left < innerWidth &&
      s.display !== "none" &&
      s.visibility !== "hidden" &&
      Number(s.opacity) > 0
    );
  };
  const check = () => {
    if (clock.stopped || clock.started == null || ++frames > 15000) return;
    const frameAt = performance.now();
    if (
      lastFrame != null &&
      frameAt - lastFrame > 100 &&
      clock.frame_gaps.length < 32
    )
      clock.frame_gaps.push({
        at_ms: frameAt - clock.started,
        duration_ms: frameAt - lastFrame,
      });
    lastFrame = frameAt;
    const value = receive.value;
    if (!value) lastValue = null;
    if (value && value !== lastValue && visible(pay) && visible(receive)) {
      lastValue = value;
      const ready = performance.now() - clock.started;
      if (clock.dom_observations.length < 32)
        clock.dom_observations.push({
          value,
          at_ms: ready,
          epoch_ms: Date.now(),
        });
      const abandon = () => {
        if (lastValue === value) lastValue = null;
      };
      requestAnimationFrame(() => {
        if (
          clock.stopped ||
          receive.value !== value ||
          !visible(pay) ||
          !visible(receive)
        ) {
          abandon();
          return;
        }
        const firstFrame = performance.now() - clock.started;
        requestAnimationFrame(() => {
          if (
            clock.stopped ||
            receive.value !== value ||
            !visible(pay) ||
            !visible(receive)
          ) {
            abandon();
            return;
          }
          if (clock.candidates.length < 32)
            clock.candidates.push({
              value,
              first_dom_ms: ready,
              first_frame_ms: firstFrame,
              input_to_paint_ms: performance.now() - clock.started,
              paint_epoch_ms: Date.now(),
            });
        });
      });
    }
    timer = requestAnimationFrame(check);
  };
  clock.stop = () => {
    clock.stopped = true;
    cancelAnimationFrame(timer);
    observer?.disconnect();
  };
  pay.addEventListener(
    "input",
    () => {
      clock.started = performance.now();
      clock.started_epoch_ms = Date.now();
      window.__liveInputStarted = clock.started;
      timer = requestAnimationFrame(check);
    },
    { once: true },
  );
}
/** Classify the captured DOM against its own browser wall clock, not the later
 * Node/CDP delivery time. A later response cannot retroactively revoke a sample. */
function eligibleQuotesAtSnapshot(events, sampleEpochMs) {
  if (!Number.isSafeInteger(sampleEpochMs) || sampleEpochMs <= 0)
    throw Error("snapshot_clock");
  const current = events.filter((e) => e.absolute_at <= sampleEpochMs).at(-1);
  const revoked =
    current?.status === 422 ||
    (current?.status === 503 && current.reason === "accounting_unavailable");
  return revoked
    ? []
    : events.filter(
        (e) =>
          e.useful && e.observed <= sampleEpochMs && e.expires > sampleEpochMs,
      );
}
/** Only quotes actually compatible with the last observed display can own its
 * retention. An older different-output response never revives an expired owner.
 * Identical visible output/metrics cannot distinguish overlapping cohorts. */
function displayedQuoteRetention(
  owners,
  events,
  sampleEpochMs,
  ownershipUnavailable = false,
) {
  const current = events.filter((e) => e.absolute_at <= sampleEpochMs).at(-1);
  if (
    current?.status === 422 ||
    (current?.status === 503 && current.reason === "accounting_unavailable")
  )
    return "revoked";
  if (ownershipUnavailable) return "unavailable";
  if (!owners.length) return "unobserved";
  const alive = owners.filter(
    (e) => e.observed <= sampleEpochMs && e.expires > sampleEpochMs,
  ).length;
  return alive === owners.length
    ? "unexpired"
    : alive === 0
      ? "expired"
      : "ambiguous";
}
function matchingDisplayedQuotes(eligible, display, human, metric) {
  return eligible.filter(
    (e) =>
      human(e.output) === display.value &&
      (e.metrics
        ? display.slippage === "Slippage " + metric(e.slippage) &&
          display.fee === "Fee " + metric(e.fee)
        : display.slippage === "Slippage Unavailable" &&
          display.fee === "Fee Unavailable"),
  );
}
/** Unchanged visible values do not prove adoption of an indistinguishable newer
 * cohort. Keep expired possible owners until a distinguishable DOM transition.
 * The cap matches the entire fixed shard read budget; overflow never proves adoption. */
function updateDisplayedQuoteOwners(previous, signature, candidates) {
  if (signature == null) return { ...previous, signature: null };
  if (signature === previous.signature && previous.unavailable) return previous;
  const possible =
    signature === previous.signature
      ? [...previous.owners, ...candidates]
      : candidates;
  const unique = new Map(
    possible.map((e) => [
      JSON.stringify([e.observed, e.expires, e.output, e.slippage, e.fee]),
      e,
    ]),
  );
  return unique.size > 600
    ? { signature, owners: [], unavailable: true }
    : { signature, owners: [...unique.values()], unavailable: false };
}
module.exports = {
  installInputClock,
  eligibleQuotesAtSnapshot,
  displayedQuoteRetention,
  matchingDisplayedQuotes,
  updateDisplayedQuoteOwners,
};
