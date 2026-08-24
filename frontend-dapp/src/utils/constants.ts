export const TERRA_LCD_URL = import.meta.env.VITE_TERRA_LCD_URL || 'https://terra-classic-lcd.publicnode.com'
export const TERRA_RPC_URL = import.meta.env.VITE_TERRA_RPC_URL || 'https://terra-classic-rpc.publicnode.com:443'
export const FACTORY_CONTRACT_ADDRESS = import.meta.env.VITE_FACTORY_ADDRESS || ''
export const ROUTER_CONTRACT_ADDRESS = import.meta.env.VITE_ROUTER_ADDRESS || ''
export const FEE_DISCOUNT_CONTRACT_ADDRESS = import.meta.env.VITE_FEE_DISCOUNT_ADDRESS || ''
export const CL8Y_TOKEN_ADDRESS =
  import.meta.env.VITE_CL8Y_TOKEN_ADDRESS || 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
export const WRAP_MAPPER_CONTRACT_ADDRESS = import.meta.env.VITE_WRAP_MAPPER_ADDRESS || ''
export const TREASURY_CONTRACT_ADDRESS = import.meta.env.VITE_TREASURY_ADDRESS || ''
export const LUNC_C_TOKEN_ADDRESS = import.meta.env.VITE_LUNC_C_TOKEN_ADDRESS || ''
export const USTC_C_TOKEN_ADDRESS = import.meta.env.VITE_USTC_C_TOKEN_ADDRESS || ''

/** Soft-launch faucet (GitLab #473) — unset hides Mint nav and shows unavailable on `/mint`. */
export const FAUCET_CONTRACT_ADDRESS = import.meta.env.VITE_FAUCET_ADDRESS || ''

/**
 * Always-on UST1 ↔ vFDUSD oracle window (GitLab #506 / parent #502).
 * Unset hides UST1 nav and shows unavailable on `/ust1`. Never overload `/mint`.
 */
export const UST1_WINDOW_CONTRACT_ADDRESS = import.meta.env.VITE_UST1_WINDOW_ADDRESS || ''
export const UST1_TOKEN_ADDRESS = import.meta.env.VITE_UST1_TOKEN_ADDRESS || ''
export const VFDUSD_TOKEN_ADDRESS = import.meta.env.VITE_VFDUSD_TOKEN_ADDRESS || ''
/** Optional — UI reads pause/staleness via window `effective_swap.oracle`. */
export const UST1_ORACLE_CONTRACT_ADDRESS = import.meta.env.VITE_UST1_ORACLE_ADDRESS || ''

/**
 * CW20 Send → ust1-window deposit/withdraw gas envelope (GitLab #506).
 * Deposit: Receive + Mint + Transfer; withdraw: Receive + Burn + treasury InstantWithdraw.
 * Ceiling matches wrap/unwrap margin discipline until LocalTerra/mainnet gas_used is pinned.
 */
export const UST1_WINDOW_SEND_GAS_LIMIT = 800_000

/**
 * CW20 Send → payee hook for DEX-routed invoices (GitLab #595).
 * Same ballpark as a router Send (600k) — launcher `EnableFeature` / settings batch
 * is one extra execute, not a hop. Combined wrap+N-hop+invoice uses this **plus**
 * the swap envelope in `totalGasLimitForExecuteMsgs`.
 */
export const PAY_INVOICE_SEND_GAS_LIMIT = 600_000

/** Inner CW20 hook keys treated as invoice settlement (#592 / #593 / #597). */
export const PAY_INVOICE_HOOK_KEYS = [
  'enable_feature',
  'create_token',
  'update_settings',
  'apply_settings',
  'subscribe',
] as const

export function isPayInvoiceHookInner(inner: Record<string, unknown>): boolean {
  return PAY_INVOICE_HOOK_KEYS.some((key) => key in inner)
}

const SOFT_LAUNCH_MINTABLE_TOKEN_ENV: { symbol: string; envKey: string }[] = [
  { symbol: 'EMBER', envKey: 'VITE_TOKEN_EMBER_ADDRESS' },
  { symbol: 'CORAL', envKey: 'VITE_TOKEN_CORAL_ADDRESS' },
  { symbol: 'JADE', envKey: 'VITE_TOKEN_JADE_ADDRESS' },
  { symbol: 'ONYX', envKey: 'VITE_TOKEN_ONYX_ADDRESS' },
  { symbol: 'RUBY', envKey: 'VITE_TOKEN_RUBY_ADDRESS' },
  { symbol: 'TOPAZ', envKey: 'VITE_TOKEN_TOPAZ_ADDRESS' },
]

export type SoftLaunchMintableToken = {
  symbol: string
  address: string
  decimals: 6
}

/** Six soft-launch mintables only (F4) — entries with empty env addresses are omitted. */
export const SOFT_LAUNCH_MINTABLE_TOKENS: SoftLaunchMintableToken[] = SOFT_LAUNCH_MINTABLE_TOKEN_ENV.map(
  ({ symbol, envKey }) => ({
    symbol,
    address: (import.meta.env[envKey as keyof ImportMetaEnv] as string | undefined) || '',
    decimals: 6 as const,
  })
).filter((t): t is SoftLaunchMintableToken => t.address.length > 0)

export function isFaucetEnabled(): boolean {
  return !!FAUCET_CONTRACT_ADDRESS
}

/**
 * True when treasury + wrap-mapper + both wrap CW20s are configured (nav + `/wrap` + Swap surface).
 * Invariant **W1** — see [`skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md`](../../skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md).
 */
export function isNativeWrapEnabled(): boolean {
  return (
    !!WRAP_MAPPER_CONTRACT_ADDRESS && !!TREASURY_CONTRACT_ADDRESS && !!LUNC_C_TOKEN_ADDRESS && !!USTC_C_TOKEN_ADDRESS
  )
}

/**
 * True when window + both token addresses are configured (nav + page execute path).
 * Invariant **U1** — see [`docs/runbooks/ust1-window-ui.md`](../../docs/runbooks/ust1-window-ui.md).
 */
export function isUst1WindowEnabled(): boolean {
  return !!UST1_WINDOW_CONTRACT_ADDRESS && !!UST1_TOKEN_ADDRESS && !!VFDUSD_TOKEN_ADDRESS
}

/**
 * Community tax create/manage (GitLab #593 / #601 / #620). Unset hides Create Token.
 * Columbus-5 Coolify: code **11619**, launcher `terra126pr5…ahzwze` (11622).
 * LocalTerra: `make deploy-local` writes the **local** store id + launcher (#620).
 * Never bake columbus-5 11611/11619 into LocalTerra `.env.local`.
 */
export const COMMUNITY_TAX_CODE_ID = Number(import.meta.env.VITE_COMMUNITY_TAX_CODE_ID || '') || 0
export const COMMUNITY_TOKEN_LAUNCHER = import.meta.env.VITE_COMMUNITY_TOKEN_LAUNCHER || ''
/** Wasm-admin banner compare. Optional override; columbus-5 CMM is the default. */
export const CMM_GOVERNANCE_ADDR =
  import.meta.env.VITE_CMM_GOVERNANCE_ADDR || 'terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2'

export function isCommunityTaxEnabled(): boolean {
  return COMMUNITY_TAX_CODE_ID > 0 && !!COMMUNITY_TOKEN_LAUNCHER
}

/** Free create (0 SKU) launcher execute — instantiate submsg (#593). */
export const COMMUNITY_CREATE_TOKEN_GAS_LIMIT = 1_200_000
/** Token `Mint` (not invoiced). */
export const COMMUNITY_MINT_GAS_LIMIT = 400_000
/** Permissionless AutoLP `SkimToLp`. */
export const COMMUNITY_SKIM_GAS_LIMIT = 800_000
/** `RegisterListedPair` factory lookup. */
export const COMMUNITY_REGISTER_PAIR_GAS_LIMIT = 400_000
/** `MsgMigrateContract` + `MsgUpdateAdmin` adopt bundle (#626). */
export const COMMUNITY_MIGRATE_ADOPT_GAS_LIMIT = 1_500_000

/** Default-branch docs in GitLab (security audit, limit orders, ADRs). */
export const DOCS_GITLAB_BASE = 'https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/blob/main/docs'

/** Public security posture (SEC-A01 / GitLab #387) — linked from dApp footer. */
export const SECURITY_POSTURE_DOC_URL = `${DOCS_GITLAB_BASE}/security-posture.md`

/** Native denom → wrapped CW20. Env keys stay `VITE_LUNC_C_*` / `VITE_USTC_C_*`; display symbols are cLUNC / cUSTC (#507). */
export const NATIVE_WRAPPED_PAIRS: Record<string, string> = {
  uluna: LUNC_C_TOKEN_ADDRESS,
  uusd: USTC_C_TOKEN_ADDRESS,
}

/** Inverse map — empty wrap env addresses are omitted (no `''` key). */
export const WRAPPED_NATIVE_PAIRS: Record<string, string> = Object.fromEntries(
  Object.entries(NATIVE_WRAPPED_PAIRS)
    .filter(([, wrapped]) => !!wrapped)
    .map(([native, wrapped]) => [wrapped, native])
)

/** Measured wrap_deposit ~301k on LocalTerra (#353). */
export const WRAP_GAS_LIMIT = 400000
/**
 * Router-mediated single-hop `execute_swap_operations` (native wrap path, CW20 send→router).
 * Direct-to-pair `swap` stays on {@link gasLimitForExecuteSwapOperations}(1) = 840k (#249).
 * Measured ~1.28M (#353).
 */
export const ROUTER_SINGLE_HOP_GAS_LIMIT = 1_400_000
/**
 * Per-hop floor for multi-hop router `execute_swap_operations`. At 900k/hop the 2-hop budget was
 * exactly 1,810,000 and live EMBER->JADE->RUBY swaps OOG deterministically (code 11, gasUsed
 * 1,810,064-1,810,206 vs 1,810,000 granted; #353), so the floor is 950k/hop for real headroom.
 */
export const ROUTER_SWAP_OPS_MIN_GAS_PER_HOP = 950_000
/**
 * Extra gas when `wrap_deposit` and router `send`→`execute_swap_operations` (N≥2)
 * share one multi-msg tx ([GitLab #587](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/587)).
 *
 * Wrap+2hop on economic hub pairs (discount registry, larger wasm state) can exceed
 * `WRAP_GAS_LIMIT + gasLimitForRouterExecuteSwapOperations(N)` because ante/wasm
 * multi-msg overhead is not in either per-msg constant. Gem-calibrated 2.31M sat
 * too close (same class as #353's 1,810,000 ceiling).
 *
 * 400k ≈ one extra wrap-class envelope — wrap+2hop stays in the tens-of-LUNC
 * fee class (~76.76 LUNC at 28.325), not hybrid 15M. Applied only for N≥2 so
 * wrap+1hop (#353) stays 1.8M.
 */
export const WRAP_ROUTER_COMBO_OVERHEAD_GAS = 400_000
/**
 * Same-msg router `execute_swap_operations` (N≥2) + `unwrap_output`
 * ([GitLab #599](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/599)).
 *
 * Analog of {@link WRAP_ROUTER_COMBO_OVERHEAD_GAS}: hop floors + mapper
 * {@link UNWRAP_GAS_LIMIT} do not include InstantWithdraw after two hub hops
 * (taxed `uusd` bank sends). Columbus-5 USTR→USTC OOGed at the 2.71M sum
 * (1.91M hops + 800k unwrap). Applied only for N≥2 so direct cUSTC→USTC
 * mapper unwrap stays 800k and router 1-hop + unwrap stays 2.2M.
 *
 * No failed `gasUsed` was attached to #599; 400k matches the wrap combo
 * class. Envelope **3,110,000** (~88.09 LUNC at 28.325) — tens-of-LUNC,
 * not hybrid 15M. Raise this named combo (not UNWRAP_GAS_LIMIT) if a
 * measured used still exceeds 3.11M.
 */
export const UNWRAP_ROUTER_COMBO_OVERHEAD_GAS = 400_000
/**
 * CW20 `send` → wrap-mapper `{ unwrap }` (and router `unwrap_output` add-on).
 *
 * Mainnet columbus-5 LCD `/cosmos/tx/v1beta1/simulate` (signer terra1xsecn…, cLUNC→mapper):
 * - unwrap amount 1 → ~387k gas_used
 * - unwrap ≥1e6 raw → **~562k** gas_used (e.g. 490e6 → 562459; failed tx
 *   `3C3B382A…287AD` wanted 550k / used 550559 OOG — ceiling was below true cost)
 * - wrap_deposit stays ~303k (WRAP_GAS_LIMIT 400k remains OK)
 *
 * Ceiling **800k** (~1.4× sim) so ReadFlat/WriteFlat headroom survives state growth.
 * Hub USTR→USTC InstantWithdraw after N≥2 hops uses {@link UNWRAP_ROUTER_COMBO_OVERHEAD_GAS}
 * on top — do not raise this floor for every unwrap (#599).
 */
export const UNWRAP_GAS_LIMIT = 800_000

export const DEV_MODE = import.meta.env.VITE_DEV_MODE === 'true'

/** Matches Station `gasPriceStep` / Terra Classic norms; used as a floor so fee math cannot underpay at broadcast (GitLab #127). */
export const MIN_GAS_PRICE_ULUNA = 28.325

const DEFAULT_GAS_PRICE_ULUNA = String(MIN_GAS_PRICE_ULUNA)

export const GAS_PRICE_ULUNA = import.meta.env.VITE_GAS_PRICE_ULUNA || DEFAULT_GAS_PRICE_ULUNA

/**
 * Gas price (uluna per gas unit) used for `Fee.amount` and wallet `gasPrice`.
 * Never below {@link MIN_GAS_PRICE_ULUNA}: a too-low `VITE_GAS_PRICE_ULUNA` causes **insufficient fees**
 * while `gas_wanted` is still high (e.g. `increase_allowance` before limit placement).
 */
export function effectiveGasPriceUluna(): number {
  const parsed = parseFloat(GAS_PRICE_ULUNA)
  const base = Number.isFinite(parsed) && parsed > 0 ? parsed : MIN_GAS_PRICE_ULUNA
  return Math.max(base, MIN_GAS_PRICE_ULUNA)
}
export const SWAP_GAS_PER_HOP = 600000
/**
 * Multiplier on (per-hop base × hop count) before floor/padding.
 * Match operational `--gas-adjustment 1.3` in `deploy-dex-local.sh` / `terrad` so the dApp is not tighter than CLI defaults.
 * GitLab #115: pool-only swap used ~753,321 gas; a 1.1× buffer produced 710k wanted and failed on-chain.
 */
export const SWAP_GAS_BUFFER = 1.3
/**
 * Minimum gas attributed per hop for `execute_swap_operations` (total floor = hops × this).
 * Guards against underestimates when buffer × base is still too low for some pairs.
 */
export const EXECUTE_SWAP_OPS_MIN_GAS_PER_HOP = 661000
/**
 * Extra gas added per hop on top of the buffered estimate (absorbs small runtime variance;
 * e.g. QA saw gasUsed 1,320,097 vs wanted 1,320,000 on a 2-hop).
 */
export const SWAP_MULTIHOP_GAS_PADDING_PER_HOP = 50000
/**
 * Flat headroom on top of buffered + padded swap gas so `gasUsed` cannot exceed `gasWanted`
 * by a few hundred units at the ceiling (GitLab #115 follow-up: 830,102 used vs 830,000 wanted).
 */
export const SWAP_GAS_SAFETY_MARGIN = 10000

/**
 * Direct pair Pattern C hybrid gas (GitLab #249; tuned after #248 transfer aggregation / #252 benchmarks;
 * book walk overhead GitLab #260 / #254).
 * `gasWanted ≈ HYBRID_SWAP_BASE_GAS + HYBRID_SWAP_PER_MAKER_GAS × (makersUsed + HYBRID_SWAP_MAKER_GAS_BUFFER)
 *   + bookWalkScanOverheadGas(...)` when `book_input > 0`,
 * clamped to [`HYBRID_SWAP_GAS_FLOOR`, `HYBRID_SWAP_GAS_LIMIT`]. Pool-only leg (`book_input = 0`) uses
 * {@link gasLimitForExecuteSwapOperations}(1) instead.
 */
export const HYBRID_SWAP_BASE_GAS = 550_000
export const HYBRID_SWAP_PER_MAKER_GAS = 65_000
export const HYBRID_SWAP_MAKER_GAS_BUFFER = 2
export const HYBRID_SWAP_GAS_FLOOR = 600_000
/**
 * Marginal gas per book-walk iteration beyond the maker-fill envelope (GitLab #260 / #254).
 * Tuned with {@link HYBRID_SWAP_GAS_LIMIT} (15M) to cover `MAX_SCAN_STEPS` (500) + parks offline worst case (GitLab #262).
 */
export const HYBRID_SWAP_PER_SCAN_STEP_GAS = 950
/** Marginal gas per expired-order park (storage write + event) on the book walk (GitLab #260 / #250). */
export const HYBRID_SWAP_PER_EXPIRED_PARK_GAS = 8_000

type NetworkConfig = {
  terra: {
    chainId: string
    lcd: string
    rpc: string
  }
}

export const NETWORKS: Record<string, NetworkConfig> = {
  local: {
    terra: {
      chainId: 'localterra',
      lcd: import.meta.env.VITE_TERRA_LCD_URL || 'http://localhost:1317',
      rpc: import.meta.env.VITE_TERRA_RPC_URL || 'http://localhost:26657',
    },
  },
  testnet: {
    terra: {
      chainId: 'rebel-2',
      lcd: 'https://terra-classic-lcd.publicnode.com',
      rpc: 'https://terra-classic-rpc.publicnode.com:443',
    },
  },
  mainnet: {
    terra: {
      chainId: 'columbus-5',
      lcd: 'https://terra-classic-lcd.publicnode.com',
      rpc: 'https://terra-classic-rpc.publicnode.com:443',
    },
  },
}

export const DEFAULT_NETWORK = (import.meta.env.VITE_NETWORK || 'local') as keyof typeof NETWORKS

/** Pair hard cap for batch cancel / claim execute msgs (`dex-common` `MAX_LIMIT_BATCH_RUNGS_HARD_CAP`). */
export const MAX_LIMIT_BATCH_RUNGS_HARD_CAP = 30

export function isValidTerraAddress(addr: string): boolean {
  return /^terra1[a-z0-9]{38,}$/.test(addr)
}
