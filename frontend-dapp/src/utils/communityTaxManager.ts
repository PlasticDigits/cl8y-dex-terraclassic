/** Connected wallet vs LCD `GetConfig.manager` — never a URL param (C593-6). */
export function isManagerWallet(connected: string | null | undefined, manager: string | undefined): boolean {
  if (!connected || !manager) return false
  return connected.trim().toLowerCase() === manager.trim().toLowerCase()
}
