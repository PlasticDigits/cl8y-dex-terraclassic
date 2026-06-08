import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

/** First dual-CW20 factory pair from deploy stamp (GitLab #292 / AGENTS_E2E_INDEXER_OUTAGE). */
export function e2eTradePairFromDeploy(): string {
  if (process.env.E2E_TRADE_PAIR) {
    return process.env.E2E_TRADE_PAIR
  }
  const stamp = path.join(__dirname, '..', '..', '..', '.qa-deploy-stamp')
  if (fs.existsSync(stamp)) {
    const m = fs.readFileSync(stamp, 'utf8').match(/^pair_address=(.+)$/m)
    if (m?.[1]) {
      return m[1].trim()
    }
  }
  throw new Error('E2E_TRADE_PAIR unset and .qa-deploy-stamp missing pair_address — run scripts/deploy-dex-local.sh')
}
