/**
 * Reject when `promise` does not settle within `timeoutMs` ([GitLab #173](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/173)).
 */
export function withPromiseTimeout<T>(promise: Promise<T>, timeoutMs: number, timeoutMessage: string): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(timeoutMessage)), timeoutMs)
    promise
      .then((value) => {
        clearTimeout(timer)
        resolve(value)
      })
      .catch((err) => {
        clearTimeout(timer)
        reject(err)
      })
  })
}
