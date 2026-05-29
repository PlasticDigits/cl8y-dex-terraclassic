/**
 * Generic CW20 wallet balance (GitLab #231).
 * Single implementation with limit escrow — shared React Query key `['tokenBalance', address, terra1…]`.
 */
export { useLimitOrderEscrowBalance as useTokenBalance } from '@/hooks/useLimitOrderEscrowBalance'
