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
    stopped: false,
  });
  let timer,
    frames = 0,
    lastValue = null;
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
    const value = receive.value;
    if (value && value !== lastValue && visible(pay) && visible(receive)) {
      lastValue = value;
      const ready = performance.now() - clock.started;
      requestAnimationFrame(() => {
        if (
          clock.stopped ||
          receive.value !== value ||
          !visible(pay) ||
          !visible(receive)
        )
          return;
        const firstFrame = performance.now() - clock.started;
        requestAnimationFrame(() => {
          if (
            clock.stopped ||
            receive.value !== value ||
            !visible(pay) ||
            !visible(receive)
          )
            return;
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
module.exports = { installInputClock, eligibleQuotesAtSnapshot };
