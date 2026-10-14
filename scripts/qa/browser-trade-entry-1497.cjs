#!/usr/bin/env node
// Read-only, disconnected entry coverage. No wallets, signing or transactions.
const fs = require("node:fs");
const path = require("node:path");
const { createHash } = require("node:crypto");
const { createRequire } = require("node:module");
const { spawnSync } = require("node:child_process");
const root = path.resolve(__dirname, "../..");
const { chromium, expect } = createRequire(
  path.join(root, "frontend-dapp/package.json"),
)("@playwright/test");
const args = process.argv.slice(2);
const option = (name, fallback) =>
  args.includes(name) ? args[args.indexOf(name) + 1] : fallback;
const origin = new URL(option("--origin", "https://dex.cl8y.com")).origin;
const output = option("--output", "/tmp/trade-entry-1497-public.json");
const receipt = {
  schema: 1,
  issue: 1497,
  started_at_utc: new Date().toISOString(),
  scope:
    "Read-only disconnected deployed Trade entry, navigation, history and reload. No connected-wallet or full outage/offline acceptance.",
  source_revision: spawnSync("git", ["rev-parse", "HEAD"], {
    cwd: root,
    encoding: "utf8",
  }).stdout.trim(),
  source_dirty: !!spawnSync("git", ["status", "--porcelain"], {
    cwd: root,
    encoding: "utf8",
  }).stdout.trim(),
  qa_helper_sha256: createHash("sha256")
    .update(fs.readFileSync(__filename))
    .digest("hex"),
  provenance_scope:
    "source_revision and source_dirty identify the local QA checkout. Deployed application source is independently attested; served asset hashes bind these browser findings to its frontend.",
  trials: [],
  served_assets: [],
  verdict: "FAIL",
};
const assets = new Map();
const factoryCatalogs = new WeakMap();
async function acknowledge(page) {
  const checkbox = page.getByRole("checkbox", {
    name: /I understand these risks/,
  });
  if (await checkbox.isVisible()) {
    await checkbox.check();
    await page.getByRole("button", { name: "Continue to the app" }).click();
  }
}
async function advanced(page, width) {
  await expect(page.getByTestId("trade-page-heading")).toBeVisible({
    timeout: 30000,
  });
  await acknowledge(page);
  await expect(
    page.getByRole("combobox", { name: "Trading pair" }),
  ).toBeEnabled({ timeout: 30000 });
  await expect(page.getByTestId("trade-order-tab-market")).toBeVisible({
    timeout: 30000,
  });
  await expect(page.getByTestId("trade-order-tab-limit")).toBeVisible();
  await expect(
    page.getByTestId(
      width >= 1024 ? "trade-desktop-chart-col" : "trade-sub-lg-chart-col",
    ),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Order book", exact: true }),
  ).toBeVisible();
  const pathname = new URL(page.url()).pathname;
  if (!/^\/trade\/terra1[a-z0-9]{38,64}$/.test(pathname))
    throw new Error("advanced-path");
  await expect
    .poll(() => factoryCatalogs.get(page)?.has(pathname.split("/").pop()), {
      timeout: 10000,
    })
    .toBe(true);
  await expect(page.getByTestId("trade-legacy-gem-notice")).toHaveCount(0);
  if (await page.getByTestId("swap-you-pay-amount").count())
    throw new Error("swap-redirect");
  return pathname;
}
async function tradeClick(page, width) {
  const nav = page.locator(
    width < 768 ? "nav.app-mobile-nav-shell" : "nav.app-desktop-nav",
  );
  if (width >= 768 && width < 1200) {
    await nav.getByRole("button", { name: /^More/ }).click();
    await nav.getByRole("menuitem", { name: "Trade", exact: true }).click();
    return;
  }
  await nav.getByRole("link", { name: "Trade", exact: true }).click();
}
async function main() {
  const browser = await chromium.launch({
    headless: true,
    timeout: 30000,
    args: ["--no-sandbox", "--disable-gpu", "--disable-dev-shm-usage"],
  });
  // Bound the entire network/browser session as well as each individual action.
  const deadline = setTimeout(() => {
    browser.close().catch(() => {});
  }, 180000);
  try {
    for (const width of [1440, 900, 390]) {
      const context = await browser.newContext({
        viewport: { width, height: 900 },
      });
      const page = await context.newPage();
      factoryCatalogs.set(page, new Set());
      page.setDefaultTimeout(10000);
      const assetTasks = [];
      page.on("response", (response) => {
        const url = new URL(response.url());
        if (url.pathname === "/api/v1/pairs" && response.ok()) {
          const task = response
            .json()
            .then((body) => {
              for (const pair of Array.isArray(body.items)
                ? body.items.slice(0, 1000)
                : []) {
                const addresses = factoryCatalogs.get(page);
                if (addresses.size >= 4096) break;
                if (
                  typeof pair.pair_address === "string" &&
                  /^terra1[a-z0-9]{38,64}$/.test(pair.pair_address)
                )
                  addresses.add(pair.pair_address);
              }
            })
            .catch(() => {});
          assetTasks.push(task);
        }
        if (
          url.origin === origin &&
          /^\/assets\/[^/]+\.js$/.test(url.pathname) &&
          response.ok()
        ) {
          const task = response
            .body()
            .then((body) =>
              assets.set(url.pathname, {
                path: url.pathname,
                sha256: createHash("sha256").update(body).digest("hex"),
                bytes: body.length,
              }),
            )
            .catch(() => {});
          assetTasks.push(task);
        }
      });
      let failed = false;
      async function trial(name, action) {
        try {
          const pathname = await action();
          receipt.trials.push({ width, name, verdict: "PASS", pathname });
        } catch {
          failed = true;
          // Browser exception text can include session/query data; fixed public copy only.
          receipt.trials.push({
            width,
            name,
            verdict: "FAIL",
            reason:
              "Trade entry/path/advanced controls did not satisfy bounded browser assertions.",
          });
        }
      }
      await trial("fresh-direct-risk-acknowledgement", async () => {
        await page.goto(origin + "/trade", {
          waitUntil: "commit",
          timeout: 30000,
        });
        return advanced(page, width);
      });
      if (!failed) {
        const selected = new URL(page.url()).pathname;
        await trial("selected-market-reload", async () => {
          await page.reload({ waitUntil: "commit", timeout: 30000 });
          const actual = await advanced(page, width);
          if (actual !== selected) throw new Error("reload-identity");
          return actual;
        });
        await trial("canonical-deep-link-market-limit-tabs", async () => {
          await page.goto(origin + selected, {
            waitUntil: "commit",
            timeout: 30000,
          });
          const actual = await advanced(page, width);
          await page.getByTestId("trade-order-tab-limit").click();
          await expect(
            page.getByTestId("trade-order-tab-limit"),
          ).toHaveAttribute("aria-selected", "true");
          await page.getByTestId("trade-order-tab-market").click();
          if (actual !== selected) throw new Error("deep-link-identity");
          return actual;
        });
        await trial("navigation-trade-click-and-repeat", async () => {
          await page.goto(origin + "/", {
            waitUntil: "commit",
            timeout: 30000,
          });
          await expect(page.getByTestId("swap-you-pay-amount")).toBeVisible({
            timeout: 30000,
          });
          await tradeClick(page, width);
          const actual = await advanced(page, width);
          for (let i = 0; i < 2; i++) {
            await tradeClick(page, width);
            if ((await advanced(page, width)) !== actual)
              throw new Error("repeat-identity");
          }
          return actual;
        });
        await trial("back-forward-keeps-trade-destination", async () => {
          await page.goBack({ waitUntil: "commit", timeout: 30000 });
          await page.goForward({ waitUntil: "commit", timeout: 30000 });
          return advanced(page, width);
        });
      }
      await Promise.allSettled(assetTasks);
      await context.close();
    }
  } finally {
    clearTimeout(deadline);
    await browser.close();
    receipt.completed_at_utc = new Date().toISOString();
    receipt.served_assets = [...assets.values()].sort((a, b) =>
      a.path.localeCompare(b.path),
    );
    receipt.verdict =
      receipt.trials.length > 0 &&
      receipt.trials.every((t) => t.verdict === "PASS")
        ? "PASS"
        : "FAIL";
    fs.writeFileSync(output, JSON.stringify(receipt, null, 2) + "\n", {
      mode: 0o600,
    });
    process.stdout.write(
      `${receipt.verdict}: ${receipt.trials.filter((t) => t.verdict === "PASS").length}/${receipt.trials.length} scoped Trade trials; receipt ${output}\n`,
    );
    if (receipt.verdict !== "PASS") process.exitCode = 1;
  }
}
main().catch(() => {
  process.stderr.write(
    "FAIL: browser helper could not complete; no full deployed acceptance.\n",
  );
  process.exitCode = 1;
});
