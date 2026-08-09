/**
 * Generic wallet balance (CW20 or native). GitLab #231.
 * Same implementation as limit escrow — shared React Query key `['tokenBalance', address, tokenId]`.
 */
export { useLimitOrderEscrowBalance as useTokenBalance } from '@/hooks/useLimitOrderEscrowBalance'
