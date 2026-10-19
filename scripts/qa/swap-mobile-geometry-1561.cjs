// Production-build mobile geometry and accessibility probes for #1561.
// Fixtures remain isolated and cannot sign, submit acceptance or broadcast.
const { createRequire } = require("node:module");
const path = require("node:path");
const frontendRequire = createRequire(
  path.resolve(__dirname, "../../frontend-dapp/package.json"),
);
const { expect } = frontendRequire("@playwright/test");

// Use measured visual viewport and navigation bounds: a visible DOM node can still
// sit behind fixed navigation. Hit-test enabled and disabled CTA faces alike.
async function surfaceGeometry(page, requireInitialFit = false) {
  const result = await page.evaluate(() => {
    const cta = document.querySelector('[data-testid="swap-submit"]');
    const nav = document.querySelector(".app-mobile-nav-shell");
    const r = cta.getBoundingClientRect();
    const vv = visualViewport;
    const visibleTop = vv?.offsetTop ?? 0;
    const visibleBottom = visibleTop + (vv?.height ?? innerHeight);
    const navRect = nav?.getBoundingClientRect();
    const headerBottom = document.querySelector(".app-top-sticky").getBoundingClientRect().bottom;
    const form = document.querySelector(".swap-page").getBoundingClientRect();
    const footer = document.querySelector(".app-footer-shell");
    const navTop =
      navRect && getComputedStyle(nav).display !== "none"
        ? navRect.top
        : visibleBottom;
    const hitPoints = [
      [r.left + r.width / 2, r.top + r.height / 2],
      [r.left + 3, r.top + r.height / 2],
      [r.right - 3, r.top + r.height / 2],
      [r.left + r.width / 2, r.top + 3],
      [r.left + r.width / 2, r.bottom - 3],
    ];
    const regions = ["swap-route-summary", "swap-quote-status"].map((id) => {
      const el = document.querySelector(`[data-testid="${id}"]`);
      const rect = el.getBoundingClientRect();
      return {
        id,
        width: el.clientWidth,
        scrollWidth: el.scrollWidth,
        height: el.clientHeight,
        scrollHeight: el.scrollHeight,
        overflowX: getComputedStyle(el).overflowX,
        overflowY: getComputedStyle(el).overflowY,
        left: rect.left,
        right: rect.right,
        specifiedGap: parseFloat(getComputedStyle(el.querySelector(".swap-route-path") ?? el).columnGap) || 0,
        visibleSegments: [...el.querySelectorAll(".swap-route-path > :not(.sr-only)")].map((part) => {
          const first = part.hasAttribute("data-token-identity") ? part.firstElementChild : part;
          const last = part.hasAttribute("data-token-identity") ? part.lastElementChild : part;
          return { left: first.getBoundingClientRect().left, right: last.getBoundingClientRect().right };
        }),
        tokenBounds: [...el.querySelectorAll("[data-token-identity]")].map(
          (token) => {
            const tokenRect = token.getBoundingClientRect();
            return {
              left: tokenRect.left,
              right: tokenRect.right,
              width: tokenRect.width,
            };
          },
        ),
      };
    });
    return {
      cta: { top: r.top, bottom: r.bottom, height: r.height, width: r.width },
      scrollY,
      visibleTop,
      visibleBottom,
      navTop,
      headerBottom,
      form: { top: form.top, bottom: form.bottom, height: form.height },
      footerDisplayed: getComputedStyle(footer).display !== "none",
      measuredNavStack: document.documentElement.style.getPropertyValue("--app-mobile-nav-stack"),
      shellPaddingBottom: getComputedStyle(document.querySelector(".app-shell")).paddingBottom,
      unobstructed: hitPoints.map(([x, y]) =>
        cta.contains(document.elementFromPoint(x, y)),
      ),
      pageWidth: document.documentElement.scrollWidth,
      viewportWidth: innerWidth,
      regions,
    };
  });
  try {
    expect(result.pageWidth, "page horizontal overflow").toBeLessThanOrEqual(
      result.viewportWidth + 1,
    );
    expect(result.footerDisplayed, "content footer is desktop-only").toBe(result.viewportWidth >= 768);
    if (requireInitialFit === true && result.viewportWidth < 768 &&
        result.form.height <= result.navTop - result.headerBottom - 16) {
      expect(Math.abs((result.form.top + result.form.bottom - result.headerBottom - result.navTop) / 2),
        "Swap centered between header and measured navigation").toBeLessThanOrEqual(1);
    }
    for (const region of result.regions) {
      expect(
        region.scrollWidth,
        `${region.id} horizontal overflow`,
      ).toBeLessThanOrEqual(region.width + 1);
      expect(
        region.scrollHeight,
        `${region.id} nested vertical overflow`,
      ).toBeLessThanOrEqual(region.height + 1);
    }
    const routeRegion = result.regions.find(
      (r) => r.id === "swap-route-summary",
    );
    if (routeRegion.visibleSegments.length) {
      const parts = routeRegion.visibleSegments;
      expect(Math.abs(parts.at(-1).right - routeRegion.right), "route right aligned").toBeLessThanOrEqual(1);
      for (let i = 1; i < parts.length; i++) {
        expect(parts[i].left - parts[i - 1].right, "compact visual route spacing").toBeLessThanOrEqual(routeRegion.specifiedGap + 1);
      }
    }
    for (const [index, token] of routeRegion.tokenBounds.entries()) {
      expect(
        token.width,
        `displayed route token ${index} retains visual width`,
      ).toBeGreaterThan(0);
      expect(
        token.left,
        `route token ${index} left inside region`,
      ).toBeGreaterThanOrEqual(routeRegion.left - 1);
      expect(
        token.right,
        `route token ${index} right inside region`,
      ).toBeLessThanOrEqual(routeRegion.right + 1);
    }
    if (requireInitialFit) {
      if (requireInitialFit === true)
        expect(result.scrollY, "initial page position").toBe(0);
      expect(
        result.cta.top,
        "CTA above visual viewport top",
      ).toBeGreaterThanOrEqual(result.visibleTop);
      expect(
        result.cta.bottom,
        "CTA above measured nav and visual viewport",
      ).toBeLessThanOrEqual(Math.min(result.navTop, result.visibleBottom));
      expect(
        result.unobstructed,
        "CTA center and inset edge midpoints receive pointer hits",
      ).toEqual([true, true, true, true, true]);
      expect(
        result.cta.height,
        "comfortable primary touch target",
      ).toBeGreaterThanOrEqual(44);
    }
  } catch (error) {
    error.measurements = result;
    throw error;
  }
  return result;
}
async function scrollActionAboveNavigation(page) {
  await page
    .getByTestId("swap-submit")
    .evaluate((el) => el.scrollIntoView({ block: "center" }));
  await page.evaluate(() => {
    const r = document
      .querySelector('[data-testid="swap-submit"]')
      .getBoundingClientRect();
    const nav = document.querySelector(".app-mobile-nav-shell");
    const navTop =
      nav && getComputedStyle(nav).display !== "none"
        ? nav.getBoundingClientRect().top
        : innerHeight;
    const bottom = Math.min(
      navTop,
      (visualViewport?.offsetTop ?? 0) +
        (visualViewport?.height ?? innerHeight),
    );
    if (r.bottom > bottom) scrollBy(0, r.bottom - bottom + 8);
  });
  return surfaceGeometry(page, "reachable");
}
async function installSyntheticWallet(context, address, providerUrl) {
  // A declared synthetic acceptance response isolates connected layout from the
  // hosted legal service; no acceptance signature or external account is used.
  await context.route("**/api/v1/signatures/status?**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        property: "dex.cl8y.com",
        latest_version: "fixture",
        signed_latest: true,
        signed_version: "fixture",
        signed_at: null,
      }),
    }),
  );
  // This isolated extension cannot sign or broadcast. It exercises the actual
  // production connection path and connected balance controls without secrets.
  await context.addInitScript(
    ({ address }) => {
      const forbidden = async () => {
        throw new Error("Synthetic geometry wallet cannot sign");
      };
      window.keplr = {
        enable: async () => {},
        experimentalSuggestChain: async () => {},
        getKey: async () => ({
          name: "Geometry fixture",
          bech32Address: address,
          pubKey: new Uint8Array([2, ...Array(32).fill(1)]),
          isNanoLedger: false,
        }),
        signAmino: forbidden,
        signDirect: forbidden,
        sendTx: forbidden,
      };
      localStorage.setItem(
        "cl8y_wallet_connection",
        JSON.stringify({ walletName: "keplr", walletType: "extension" }),
      );
    },
    { address },
  );
  await context.route(`${providerUrl}/**`, async (route) => {
    const url = new URL(route.request().url());
    let body;
    if (url.pathname.includes("/bank/"))
      body = { balance: { denom: "uluna", amount: "1000000000000" } };
    else if (url.pathname.includes("/smart/")) {
      const query = JSON.parse(
        Buffer.from(
          decodeURIComponent(url.pathname.split("/smart/")[1]),
          "base64",
        ).toString(),
      );
      if (query.balance) body = { data: { balance: "1000000000000" } };
      else body = { data: {} };
    } else body = {};
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(body),
    });
  });
}
async function assertAccessible(page) {
  const { default: AxeBuilder } = createRequire(
    path.resolve(__dirname, "../../frontend-dapp/package.json"),
  )("@axe-core/playwright");
  const scan = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
    .analyze();
  const violations = scan.violations.filter((v) =>
    ["critical", "serious"].includes(v.impact),
  );
  expect(
    violations.map((v) => ({
      id: v.id,
      nodes: v.nodes.map((n) => n.target),
    })),
  ).toEqual([]);

  return { critical_or_serious: violations.length };
}
async function verifyCompactStates({
  page,
  check,
  width,
  height,
  fixture,
  ready,
  pay,
  details,
}) {
  if (width < 768) {
    await check("mobile-footer-hidden-and-about-accessible-in-more", async () => {
      await page.getByRole("button", { name: "More", exact: true }).click();
      const about = page.getByRole("region", { name: "About and legal" });
      await expect(about).toBeVisible();
      await expect(about.getByRole("link", { name: "Homepage", exact: true })).toBeVisible();
      await expect(about.getByRole("link", { name: "Security", exact: true })).toBeVisible();
      await page.getByRole("button", { name: "More", exact: true }).click();
      await expect(about).toHaveCount(0);
      return { footer_displayed: false, environment_product_and_legal_available_in_more: true };
    });
  }
  if (process.env.SWAP_GEOMETRY_AXE === "1") {
    await check("axe-critical-serious-and-keyboard-focus", async () => {
      const accessibility = await assertAccessible(page);
      await pay.focus();
      await expect(pay).toBeFocused();
      await page.keyboard.press("Tab");
      const focus = await page.evaluate(() => {
        const el = document.activeElement;
        const style = getComputedStyle(el);
        return {
          tag: el.tagName,
          name: el.getAttribute("aria-label") || el.textContent.trim(),
          outline: style.outlineStyle,
          shadow: style.boxShadow,
        };
      });
      expect(focus.tag).not.toBe("BODY");
      expect(
        focus.outline !== "none" || focus.shadow !== "none",
        "keyboard focus has visible styling",
      ).toBe(true);
      return { ...accessibility, focus };
    });
  }
  await check("primary-action-keyboard-and-wallet-modal", async () => {
    const cta = page.getByTestId("swap-submit");
    await expect(cta).toBeEnabled();
    await cta.focus();
    await expect(cta).toBeFocused();
    if (process.env.SWAP_GEOMETRY_CONNECTED !== "1") {
      await page.keyboard.press("Enter");
      const modal = page.getByRole("dialog");
      await expect(modal).toBeVisible();
      await page.keyboard.press("Escape");
      await expect(modal).toHaveCount(0);
      await expect(cta).toBeFocused();
    }
    return {
      action_enabled: true,
      wallet_modal:
        process.env.SWAP_GEOMETRY_CONNECTED !== "1"
          ? "keyboard open/close"
          : "synthetic connection already active",
    };
  });
  await check(
    "concise-current-error-remains-inline-and-unobscured",
    async () => {
      fixture.set({ mode: "rejection" });
      await pay.fill("2");
      const status = page.getByTestId("swap-quote-status");
      await expect(status).toContainText("This trade is not supported");
      await expect(status.locator(".swap-status-cross")).toHaveCount(1);
      await expect(status.locator(".swap-status-cross")).toHaveAttribute(
        "aria-hidden",
        "true",
      );
      await expect(
        status.getByRole("button", { name: /close|dismiss/i }),
      ).toHaveCount(0);
      const chrome = await status.evaluate((el) => {
        const card = el.querySelector(".alert-error") ?? el;
        const style = getComputedStyle(card);
        const cross = el
          .querySelector(".swap-status-cross")
          .getBoundingClientRect();
        const text = [...el.querySelectorAll("p")].find((p) =>
          p.textContent.trim(),
        );
        const range = document.createRange();
        range.selectNodeContents(text ?? el);
        const line = range.getClientRects()[0];
        return {
          padding: style.padding,
          border: style.borderWidth,
          background: style.backgroundColor,
          crossLineDelta: Math.abs(cross.top - line.top),
        };
      });
      expect(chrome.padding).toBe("0px");
      expect(chrome.border).toBe("0px");
      expect(chrome.background).toBe("rgba(0, 0, 0, 0)");
      expect(
        chrome.crossLineDelta,
        "decorative cross shares first error line",
      ).toBeLessThanOrEqual(3);
      await page.evaluate(() => scrollTo(0, 0));
      const bounds = await surfaceGeometry(
        page,
        width === 375 && height === 667,
      );
      const accessibility =
        process.env.SWAP_GEOMETRY_AXE === "1"
          ? await assertAccessible(page)
          : undefined;
      fixture.set();
      await pay.fill("3");
      await ready(page, "3");
      return { ...bounds, chrome, accessibility };
    },
  );
  await check(
    "measured-navigation-clearance-and-shortened-visual-viewport",
    async () => {
      await page.locator(".app-mobile-nav-shell").evaluate((el) => {
        el.style.paddingBottom = "20px";
      });
      await page.waitForTimeout(50); // allow the owned navigation ResizeObserver to publish its new stack
      await page.evaluate(() => scrollTo(0, 0));
      const inset = await surfaceGeometry(
        page,
        width === 375 && height === 667,
      );
      await page.setViewportSize({
        width,
        height: Math.min(height, 420),
      });
      const shortened = await scrollActionAboveNavigation(page);
      await page.setViewportSize({ width, height });
      await page.locator(".app-mobile-nav-shell").evaluate((el) => {
        el.style.paddingBottom = "";
      });
      await page.waitForTimeout(50);
      return {
        inset_simulation_px: 20,
        inset,
        shortened_viewport: shortened,
      };
    },
  );
  await check(
    "200-percent-text-vertical-reflow-and-keyboard-reachability",
    async () => {
      await page.evaluate(() => {
        document.documentElement.style.fontSize = "200%";
      });
      await surfaceGeometry(page);
      await page.getByTestId("swap-submit").scrollIntoViewIfNeeded();
      // Keyboard focus can be on the disclosure even when the genuine blocker disables CTA.
      await details.locator("summary").focus();
      await expect(details.locator("summary")).toBeFocused();
      await page.keyboard.press("Enter");
      await expect(details).toHaveAttribute("open", "");
      return scrollActionAboveNavigation(page);
    },
  );
}
// Presentation-only probes retain a valid blocked quote; the longer sentence
// replaces only its DOM text to reproduce the reported wrapping, never economics.
async function verifyCompactStatus(page) {
  const status = page.getByTestId("swap-quote-status");
  const notice = page.getByTestId("swap-impact-aside");
  const action = notice.getByRole("button", {name: "Enable Expert Mode", exact: true});
  await expect(action).toBeVisible();
  const results = [];
  for (const sentence of [null, "Order slippage 63.28%; fair-reference deviation 64.91% requires Expert Mode."]) {
    if (sentence) await notice.locator("p").evaluate((p, text) => { p.firstChild.textContent = text; }, sentence);
    const result = await status.evaluate(el => {
      const button = el.querySelector("button");
      const r = button.getBoundingClientRect();
      const p = button.closest("p");
      const range = document.createRange(); range.selectNodeContents(p.firstChild);
      const rects = [...range.getClientRects()];
      const style = getComputedStyle(el), bs = getComputedStyle(button);
      const hitHeight = parseFloat(getComputedStyle(button, "::after").height);
      const cx = r.left + r.width/2, cy = r.top+r.height/2;
      const details = document.querySelector('[data-testid="swap-details"] summary').getBoundingClientRect();
      return {height:el.getBoundingClientRect().height, font:parseFloat(style.fontSize), lineHeight:parseFloat(style.lineHeight), minHeight:parseFloat(style.minHeight),
        actionHeight:r.height, hitHeight, hitWidth:r.width, actionBottom:cy+hitHeight/2, detailsTop:details.top,
        textLineTops:rects.map(x=>x.top), hits:[cy-21,cy,cy+21].map(y=>button.contains(document.elementFromPoint(cx,y)))};
    });
    expect(result.lineHeight/result.font, "compact text leading").toBeLessThanOrEqual(1.4);
    expect(result.minHeight/result.font, "compact reserved height on every breakpoint").toBeLessThanOrEqual(2.71);
    expect(result.actionHeight, "inline action does not inflate text line boxes").toBeLessThanOrEqual(result.lineHeight+1);
    expect(result.hitHeight, "motor-accessible action hit area").toBeGreaterThanOrEqual(44);
    expect(result.hitWidth).toBeGreaterThanOrEqual(44);
    expect(result.hits).toEqual([true,true,true]);
    expect(result.actionBottom, "action target clear of DETAILS").toBeLessThanOrEqual(result.detailsTop+1);
    for (let i=1;i<result.textLineTops.length;i++) expect(result.textLineTops[i]-result.textLineTops[i-1]).toBeLessThanOrEqual(result.lineHeight+1);
    results.push(result);
  }
  await action.focus(); await page.keyboard.press("Enter");
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.keyboard.press("Escape"); await expect(action).toBeFocused();
  const accessibility = await assertAccessible(page);
  return {cases:results, accessibility, expert_action_keyboard:"opens modal and returns focus without enabling or signing"};
}
async function verifyOrdinaryReview({ page, fixture, pay, ready, connected }) {
  expect(connected, "ordinary review uses an isolated connected fixture").toBe(
    true,
  );
  const cta = page.getByTestId("swap-submit");
  await expect(page.getByTestId("swap-quote-status")).toContainText(
    "Fair-reference deviation 11.00%",
  );
  await expect(cta).toBeEnabled();
  const minimum = await page
    .getByTestId("swap-confirm-min-return")
    .textContent();
  const original = fixture.observations().at(-1);
  expect(original.generation).toBeTruthy();
  expect(original.expires_at_ms).toBeGreaterThan(Date.now());
  await cta.click();
  await expect(cta).toHaveText("Confirm Swap");
  const requests = fixture.requests();
  await page.waitForTimeout(12000);
  expect(
    fixture.producerObservedAt(),
    "independent producer advances during review",
  ).toBeGreaterThan(original.observed_at_ms);
  expect(
    fixture.requests(),
    "automatic HTTP quote refresh pauses during original review",
  ).toBe(requests);
  expect(
    fixture.observations().at(-1),
    "original generation/expiry response remains unchanged",
  ).toEqual(original);
  await expect(cta).toHaveText("Confirm Swap");
  await expect(cta).toBeEnabled();
  await expect(page.getByTestId("swap-confirm-min-return")).toHaveText(minimum);
  await pay.fill("2");
  await ready(page, "2");
  await expect(cta).toHaveText("Swap");
  expect(
    fixture.requests(),
    "explicit intent edit owns a new quote and resets consent",
  ).toBeGreaterThan(requests);
  return {
    original_response: original,
    producer_observed_at_ms_after_review: fixture.producerObservedAt(),
    paused_for_ms: 12000,
    quote_requests_during_review: 0,
    changed_intent_resets_review: true,
    signing_or_broadcast_attempts: 0,
  };
}
module.exports = {
  surfaceGeometry,
  installSyntheticWallet,
  verifyCompactStates,
  verifyOrdinaryReview,
  verifyCompactStatus,
};
