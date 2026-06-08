import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import { isChainOptional } from './chain'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

/** Legacy first dual-CW20 pair when optional-chain smoke loads specs without deploy stamp. */
const LEGACY_FALLBACK_PAIR = 'terra10y4jzxavk0uw2usy7ezt4dq5h0k64na8c9yz3rq3dk50v7j8mezs89tz96'

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
  if (isChainOptional()) {
    return LEGACY_FALLBACK_PAIR
  }
  throw new Error('E2E_TRADE_PAIR unset and .qa-deploy-stamp missing pair_address — run scripts/deploy-dex-local.sh')
}
