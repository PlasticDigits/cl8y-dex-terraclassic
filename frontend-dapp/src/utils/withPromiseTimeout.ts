/**
 * Reject when `promise` does not settle within `timeoutMs` ([GitLab #173](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/173)).
 * A late settlement after timeout is ignored so hung Keplr/Ledger `signAmino` cannot broadcast after the UI gave up ([GitLab #567](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/567)).
 */
export function withPromiseTimeout<T>(promise: Promise<T>, timeoutMs: number, timeoutMessage: string): Promise<T> {
  return new Promise((resolve, reject) => {
    let settled = false
    const timer = setTimeout(() => {
      if (settled) return
      settled = true
      reject(new Error(timeoutMessage))
    }, timeoutMs)
    promise
      .then((value) => {
        if (settled) return
        settled = true
        clearTimeout(timer)
        resolve(value)
      })
      .catch((err) => {
        if (settled) return
        settled = true
        clearTimeout(timer)
        reject(err)
      })
  })
}
