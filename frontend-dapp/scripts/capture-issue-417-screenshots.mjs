import { chromium } from '@playwright/test'

const OUT = '/home/agent/workspace/docs/screenshots/issue-417'

async function main() {
  const browser = await chromium.launch()
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } })
  await page.addInitScript(() => {
    window.localStorage.removeItem('cl8y-dex-trade-onboarding-dismissed')
    window.localStorage.removeItem('cl8y-dex-trade-tape-expanded')
    window.localStorage.removeItem('cl8y-dex-trade-wallet-history-expanded')
  })

  await page.goto('http://127.0.0.1:5173/trade')
  await page.getByTestId('trade-onboarding-strip').waitFor({ timeout: 60000 })
  await page.locator('[data-testid="trade-desktop-workspace"], [data-testid="trade-sub-lg-workspace"]').first().waitFor({ timeout: 60000 })
  await page.getByTestId('trade-limit-submit').first().waitFor({ timeout: 60000 })
  const ticket = page.locator('[data-testid="trade-sub-lg-ticket-col"]').first()
  if (await ticket.count()) {
    await ticket.screenshot({ path: `${OUT}/trade-ticket-ctas.png` })
  } else {
    await page.getByTestId('trade-limit-submit').first().locator('xpath=ancestor::div[contains(@class,"card-glass")]').first().screenshot({ path: `${OUT}/trade-ticket-ctas.png` })
  }

  await page.goto('http://127.0.0.1:5173/')
  await page.getByTestId('trade-onboarding-strip').waitFor({ timeout: 60000 })
  await page.locator('.shell-panel-strong').screenshot({ path: `${OUT}/swap-cta-reference.png` })

  await browser.close()
}

main().catch((err) => {
  console.error(err)
  process.exit(1)
})
