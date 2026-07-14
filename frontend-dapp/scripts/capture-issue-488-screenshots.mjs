/**
 * Visual QA capture for GitLab #488 reopen.
 * Run: node scripts/capture-issue-488-screenshots.mjs
 * Requires Vite on http://127.0.0.1:5173
 */
import { chromium } from 'playwright'
import { mkdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const OUT = join(__dirname, '../../docs/qa/issue-488')
const BASE = process.env.CAPTURE_BASE_URL || 'http://127.0.0.1:5173'

mkdirSync(OUT, { recursive: true })

const pages = [
  { path: '/', name: 'swap' },
  { path: '/limits', name: 'limits' },
  { path: '/pool', name: 'pool' },
  { path: '/trade', name: 'trade' },
  { path: '/portfolio', name: 'portfolio' },
  { path: '/charts', name: 'charts' },
  { path: '/tiers', name: 'tiers' },
  { path: '/create', name: 'create' },
]

async function dismissRiskAck(page) {
  await page.evaluate(() => {
    localStorage.setItem('cl8y-dex-risk-ack', '1')
    localStorage.setItem('cl8y-risk-acknowledged', '1')
  })
  // Common keys used by RiskAcknowledgementModal — set both variants
  const keys = await page.evaluate(() => Object.keys(localStorage))
  for (const k of keys) {
    if (/risk|ack|legal/i.test(k)) {
      /* keep */
    }
  }
}

async function setTheme(page, theme) {
  await page.evaluate((t) => {
    localStorage.setItem('cl8y-dex-theme', t)
    document.documentElement.setAttribute('data-theme', t)
  }, theme)
}

async function shot(page, file) {
  await page.waitForTimeout(900)
  await page.screenshot({ path: join(OUT, file), fullPage: true, type: 'jpeg', quality: 82 })
  console.log('wrote', file)
}

async function main() {
  const browser = await chromium.launch({
    headless: true,
    args: ['--no-sandbox', '--disable-dev-shm-usage'],
  })
  const context = await browser.newContext({
    viewport: { width: 1280, height: 900 },
    deviceScaleFactor: 1,
  })
  const page = await context.newPage()

  // Discover risk-ack storage key on first load
  await page.goto(`${BASE}/`, { waitUntil: 'domcontentloaded', timeout: 60000 })
  await page.waitForTimeout(500)
  const ackBtn = page.getByRole('button', { name: /acknowledge|i understand|accept|continue/i })
  if (await ackBtn.count()) {
    // Capture risk modal first (dark)
    await setTheme(page, 'dark')
    await page.reload({ waitUntil: 'domcontentloaded' })
    await page.waitForTimeout(600)
    await page.screenshot({ path: join(OUT, 'risk-ack-modal-dark.jpg'), type: 'jpeg', quality: 82 })
    console.log('wrote risk-ack-modal-dark.jpg')
    await ackBtn.first().click().catch(() => {})
  }
  await dismissRiskAck(page)

  for (const theme of ['dark', 'light']) {
    await setTheme(page, theme)
    for (const { path, name } of pages) {
      await page.goto(`${BASE}${path}`, { waitUntil: 'networkidle', timeout: 90000 }).catch(async () => {
        await page.goto(`${BASE}${path}`, { waitUntil: 'domcontentloaded', timeout: 60000 })
      })
      await page.waitForTimeout(1200)
      // Dismiss risk if it reappears
      const again = page.getByRole('button', { name: /acknowledge|i understand|accept|continue/i })
      if (await again.count()) await again.first().click().catch(() => {})
      await shot(page, `${name}-${theme}.jpg`)
    }

    if (theme === 'dark') {
      // Header brand crop
      await page.goto(`${BASE}/`, { waitUntil: 'domcontentloaded' })
      await page.waitForTimeout(800)
      const header = page.locator('.app-header, header').first()
      if (await header.count()) {
        await header.screenshot({ path: join(OUT, 'header-brand-dark.png') })
        console.log('wrote header-brand-dark.png')
      }

      // Wallet modal
      const connect = page.getByRole('button', { name: /connect wallet/i }).first()
      if (await connect.count()) {
        await connect.click()
        await page.waitForTimeout(500)
        await page.screenshot({ path: join(OUT, 'wallet-modal-dark.jpg'), type: 'jpeg', quality: 82 })
        console.log('wrote wallet-modal-dark.jpg')
        await page.keyboard.press('Escape')
      }
    }
  }

  // Favicon + OG copies for issue upload
  await page.goto(`${BASE}/favicon-32.png`)
  // just note paths exist on disk
  console.log('public assets: favicon-32.png, og-image.png under frontend-dapp/public/')
  await browser.close()
}

main().catch((err) => {
  console.error(err)
  process.exit(1)
})
