# Agent playbook: Transaction toast notifications

Use when changing floating tx feedback ([GitLab **#351**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/351)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [`ToastContext.tsx`](../frontend-dapp/src/contexts/ToastContext.tsx) | `ToastProvider`, viewport portal |
| [`toastContextState.ts`](../frontend-dapp/src/contexts/toastContextState.ts) | `useToast`, `useOptionalToast`, `toastErrorMessage` |
| [`useTerraBroadcastMutation.ts`](../frontend-dapp/src/hooks/useTerraBroadcastMutation.ts) | Optional `toastSuccess` + default error toasts |
| [`TxResultAlert.tsx`](../frontend-dapp/src/components/ui/TxResultAlert.tsx) | Inline banners remain on forms |

## Invariants

1. **Dual feedback:** Floating toasts supplement — do not replace — inline `TxResultAlert` on the action surface.
2. **Errors:** Use `toastErrorMessage` / `humanizeUserFacingError` for toast copy.
3. **Provider:** `ToastProvider` wraps the app in [`App.tsx`](../frontend-dapp/src/App.tsx).
4. **Broadcast txs:** Pass `toastSuccess: '…'` to `useTerraBroadcastMutation`; errors toast by default (`toastOnError`, default `true`).
5. **Plain mutations:** `useLimitOrderCancelMutation`, `useLimitOrderUpdatePriceMutation`, etc. call `useOptionalToast()` so unit tests without the provider still pass.

## Tests

```bash
cd frontend-dapp && npm test -- src/contexts/__tests__/ToastContext.test.tsx
```
