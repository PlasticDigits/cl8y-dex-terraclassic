// Geometry checks run after initialized timing trials; they never reset those clocks.
const { createRequire } = require("node:module");
const path = require("node:path");
const { expect } = createRequire(
  path.resolve(__dirname, "../../frontend-dapp/package.json"),
)("@playwright/test");
const IDS = [
  "swap-you-pay-amount",
  "swap-you-receive",
  "swap-quote-summary",
  "swap-quote-status",
];
async function geometry(page) {
  return page.evaluate(
    (ids) =>
      Object.fromEntries(
        ids.map((id) => {
          const element = document.querySelector(`[data-testid="${id}"]`);
          if (!element) throw new Error(`Missing geometry target ${id}`);
          const r = element.getBoundingClientRect();
          return [
            id,
            {
              x: r.x + scrollX,
              y: r.y + scrollY,
              width: r.width,
              height: r.height,
            },
          ];
        }),
      ),
    IDS,
  );
}
async function stable(page, baseline, label) {
  const actual = await geometry(page);
  for (const id of IDS) {
    for (const key of ["x", "y", "width", "height"]) {
      expect(
        Math.abs(actual[id][key] - baseline[id][key]),
        `${label}: ${id}.${key}`,
      ).toBeLessThanOrEqual(1);
    }
  }
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth + 1,
    ),
    `${label}: page overflow`,
  ).toBe(true);
  await expect(page.getByTestId("swap-details")).toHaveAttribute("open", "");
  await expect(
    page.locator("details > summary").filter({ hasText: /^DETAILS$/ }),
  ).toHaveCount(1);
  await expect(page.getByTestId("swap-network-fee")).toHaveCount(1);
  return actual;
}
async function nextRequest(fixture, before) {
  const deadline = Date.now() + 15000;
  while (fixture.requests() <= before && Date.now() < deadline)
    await new Promise((resolve) => setTimeout(resolve, 25));
  expect(
    fixture.requests(),
    "scheduled local HTTP quote refresh",
  ).toBeGreaterThan(before);
}
async function ready(page, amount) {
  await expect(page.getByTestId("swap-you-receive")).toHaveValue(amount, {
    timeout: 15000,
  });
  await expect(page.getByTestId("swap-price-data-loading")).toHaveCount(0);
}
async function verifySwapGeometry({ browser, origin, from, to, fixture }) {
  const receipts = [];
  for (const width of [1440, 320, 390, 430]) {
    process.stderr.write(`geometry ${width}: initialize production fixture\n`);
    const context = await browser.newContext({
      viewport: { width, height: 1000 },
      reducedMotion: "reduce",
    });
    const page = await context.newPage();
    await page.bringToFront();
    page.setDefaultTimeout(15000);
    const findings = [];
    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(error.message));
    async function check(name, action) {
      process.stderr.write(`geometry ${width}: ${name}\n`);
      try {
        findings.push({ name, verdict: "PASS", ...(await action()) });
      } catch (error) {
        const state = await page
          .evaluate(() => ({
            recorded_at_ms: Date.now(),
            visibility: document.visibilityState,
            receive:
              document.querySelector('[data-testid="swap-you-receive"]')
                ?.value ?? null,
            minimum:
              document.querySelector('[data-testid="swap-confirm-min-return"]')
                ?.textContent ?? null,
            status:
              document
                .querySelector('[data-testid="swap-quote-status"]')
                ?.textContent?.trim() ?? null,
            loading: !!document.querySelector(
              '[data-testid="swap-price-data-loading"]',
            ),
          }))
          .catch(() => null);
        findings.push({
          name,
          verdict: "FAIL",
          reason: String(error.message).slice(0, 1200),
          state,
        });
      }
    }
    try {
      fixture.set();
      await page.goto(`${origin}/?from=${from}&to=${to}`, {
        waitUntil: "domcontentloaded",
        timeout: 30000,
      });
      const pay = page.getByTestId("swap-you-pay-amount");
      await page.waitForFunction(
        () =>
          document.querySelector('[data-testid="swap-you-pay-amount"]') ||
          document.querySelector('input[type="checkbox"]'),
        undefined,
        { timeout: 30000 },
      );
      const risk = page.getByRole("checkbox", {
        name: /I understand these risks/,
      });
      if (await risk.isVisible()) {
        await risk.check();
        await page.getByRole("button", { name: "Continue to the app" }).click();
      }
      await pay.waitFor({ timeout: 30000 });
      expect(await page.evaluate(() => document.visibilityState)).toBe(
        "visible",
      );
      await pay.fill("1");
      await ready(page, "1");
      await page.evaluate(() => document.fonts.ready);
      const details = page.getByTestId("swap-details");
      const toggle = details.locator("summary");
      await check(
        "keyboard-single-details-disconnected-minimum-and-network-fee",
        async () => {
          await toggle.focus();
          await expect(toggle).toBeFocused();
          await page.keyboard.press("Enter");
          await expect(details).toHaveAttribute("open", "");
          await expect(toggle).toBeFocused();
          await expect(
            page.getByTestId("swap-confirm-min-return"),
          ).toContainText("0.95");
          await expect(page.getByTestId("swap-network-fee")).toHaveCount(1);
          await expect(page.getByTestId("swap-network-fee")).toContainText(
            "LUNC",
          );
          await expect(page.getByTestId("swap-submit")).toContainText(
            /Connect/i,
          );
          await expect(
            page.getByText("Quote only", { exact: true }),
          ).toHaveCount(0);
          await expect(
            page.getByText(/If your wallet shows a much larger fee/),
          ).toHaveCount(0);
          await expect(
            page.getByText(/This swap will wrap your tokens/),
          ).toHaveCount(0);
          await expect(page.getByTestId("swap-status-none")).toContainText(
            "None",
          );
          const route = page.getByTestId("swap-route-summary");
          expect(
            await route
              .locator("[data-token-identity]")
              .evaluateAll((elements) =>
                elements.map((el) => el.dataset.tokenIdentity),
              ),
          ).toEqual([from, to]);
          await route.focus();
          await expect(route).toBeFocused();
          const summaryBox = await page
            .getByTestId("swap-quote-summary")
            .boundingBox();
          const payBox = await pay.boundingBox();
          const receiveBox = await page
            .getByTestId("swap-you-receive")
            .boundingBox();
          expect(summaryBox.y).toBeGreaterThan(
            receiveBox.y + receiveBox.height,
          );
          expect(receiveBox.y).toBeGreaterThan(payBox.y + payBox.height);
          return {
            minimum: await page
              .getByTestId("swap-confirm-min-return")
              .innerText(),
          };
        },
      );
      // Guarantee open even when a preceding assertion fails; retain the failed verdict.
      if (!(await details.evaluate((el) => el.open))) await toggle.click();
      const baseline = await geometry(page);
      await check(
        "delayed-same-intent-refresh-retains-estimate-summary-and-open-details",
        async () => {
          fixture.set({ delayMs: 1800 });
          const completedBefore = fixture.completed();
          await nextRequest(fixture, fixture.requests());
          await page.waitForTimeout(150);
          await expect(page.getByTestId("swap-you-receive")).toHaveValue("1");
          await expect(page.getByTestId("swap-price-data-loading")).toHaveCount(
            0,
          );
          await stable(page, baseline, "fresh background refresh");
          await expect
            .poll(() => fixture.completed(), { timeout: 5000 })
            .toBeGreaterThan(completedBefore);
          await ready(page, "1");
        },
      );
      await check(
        "authoritative-rejection-keeps-input-status-summary-geometry",
        async () => {
          fixture.set({ mode: "rejection" });
          await nextRequest(fixture, fixture.requests());
          await expect(page.getByTestId("swap-quote-status")).toContainText(
            "This trade is not supported",
            { timeout: 15000 },
          );
          await expect(page.getByTestId("swap-you-receive")).toHaveValue("");
          await expect(
            page.getByTestId("swap-confirm-min-return"),
          ).toContainText("—");
          await stable(page, baseline, "authoritative rejection");
        },
      );
      for (const [mode, amount] of [
        ["missing", "2"],
        ["stale", "3"],
      ]) {
        await check(
          `${mode}-publication-equal-height-indeterminate-summary`,
          async () => {
            fixture.set({ mode });
            await pay.fill(amount);
            await expect(
              page.getByTestId("swap-price-data-loading"),
            ).toBeVisible();
            await expect(
              page.getByRole("progressbar", { name: "Loading price" }),
            ).toBeVisible();
            await stable(page, baseline, `${mode} publication`);
          },
        );
      }
      await check(
        "recovery-new-intent-restores-supported-disconnected-floor",
        async () => {
          fixture.set();
          await pay.fill("4");
          await ready(page, "4");
          await expect(
            page.getByTestId("swap-confirm-min-return"),
          ).toContainText("3.8");
          await stable(page, baseline, "recovery");
        },
      );
      await check(
        "original-expiry-replaces-summary-with-equal-height-wait",
        async () => {
          fixture.set({ pauseProducer: true, observedAgeMs: 18000 });
          await pay.fill("5");
          await ready(page, "5");
          fixture.set({
            mode: "stale",
            pauseProducer: true,
            observedAgeMs: 19000,
          });
          await expect(page.getByTestId("swap-price-data-loading")).toBeVisible(
            { timeout: 5000 },
          );
          await expect(
            page.getByTestId("swap-confirm-min-return"),
          ).toContainText("—");
          await stable(page, baseline, "original expiry");
        },
      );
      await check(
        "rapid-new-intent-loading-keeps-fixed-inputs-and-disclosure",
        async () => {
          fixture.set({ delayMs: 800 });
          await pay.fill("6");
          await expect(
            page.getByTestId("swap-price-data-loading"),
          ).toBeVisible();
          await stable(page, baseline, "new intent");
          await pay.fill("7");
          await ready(page, "7");
          await stable(page, baseline, "latest intent");
        },
      );
      await check(
        "200-percent-root-text-reduced-motion-overflow-focus-and-row-alignment",
        async () => {
          await page.evaluate(() => {
            document.documentElement.style.fontSize = "200%";
          });
          expect(
            await page.evaluate(
              () => matchMedia("(prefers-reduced-motion: reduce)").matches,
            ),
          ).toBe(true);
          const zoomBaseline = await geometry(page);
          await stable(page, zoomBaseline, "200% root text");
          const rows = await page
            .locator(
              '[data-testid="swap-confirm-max-spread"], [data-testid="swap-deadline-summary"], [data-testid="swap-confirm-min-return"], [data-testid="swap-network-fee"]',
            )
            .evaluateAll((elements) =>
              elements.map((el) => {
                const rects = [...el.children].map((child) =>
                  child.getBoundingClientRect(),
                );
                const lines = [...el.children].map((child) => {
                  const walker = document.createTreeWalker(
                    child,
                    NodeFilter.SHOW_TEXT,
                  );
                  let text = walker.nextNode();
                  while (text && !text.textContent.trim())
                    text = walker.nextNode();
                  const range = document.createRange();
                  range.selectNodeContents(text ?? child);
                  return range.getClientRects()[0];
                });
                return {
                  bottomDelta: Math.abs(rects[0].bottom - rects[1].bottom),
                  firstLineDelta: Math.abs(lines[0].bottom - lines[1].bottom),
                  overlap: rects[0].right > rects[1].left + 1,
                };
              }),
            );
          expect(rows.length).toBe(4);
          expect(
            rows.every((row) => !row.overlap),
            "label/value do not overlap",
          ).toBe(true);
          expect(
            rows.every((row) => row.firstLineDelta <= 4),
            "first label/value text lines align",
          ).toBe(true);
          await toggle.focus();
          await page.keyboard.press("Space");
          await expect(details).not.toHaveAttribute("open", "");
          await expect(toggle).toBeFocused();
          await page.keyboard.press("Enter");
          await expect(details).toHaveAttribute("open", "");
          return { text_size: "200% root font size", rows };
        },
      );
      receipts.push({
        width,
        verdict:
          findings.every((f) => f.verdict === "PASS") && !pageErrors.length
            ? "PASS"
            : "FAIL",
        findings,
        page_errors: pageErrors,
        fixture_observations: fixture.observations(),
      });
    } catch (error) {
      process.stderr.write(
        `geometry ${width}: setup failed ${String(error.message).slice(0, 400)}\n`,
      );
      receipts.push({
        width,
        verdict: "FAIL",
        findings,
        reason: String(error.message).slice(0, 1200),
        page_errors: pageErrors,
      });
    } finally {
      fixture.set();
      await context.close();
    }
  }
  return receipts;
}
module.exports = { verifySwapGeometry };
