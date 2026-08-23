/** Columbus-5 Create Token pins (GitLab #602). Bake so e2e-smoke sees /token/create. */
export const MAINNET_COMMUNITY_TAX_CODE_ID = '11611'
export const MAINNET_COMMUNITY_TOKEN_LAUNCHER = 'terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze'

export const COMMUNITY_TAX_E2E_VITE_ENV = {
  VITE_COMMUNITY_TAX_CODE_ID: process.env.VITE_COMMUNITY_TAX_CODE_ID?.trim() || MAINNET_COMMUNITY_TAX_CODE_ID,
  VITE_COMMUNITY_TOKEN_LAUNCHER: process.env.VITE_COMMUNITY_TOKEN_LAUNCHER?.trim() || MAINNET_COMMUNITY_TOKEN_LAUNCHER,
} as const
