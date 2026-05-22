/** Prefill payload from the trade order book “Edit” row action (replace-style UX; GitLab #162, #178). */
export type LimitBookTicketDraft = {
  side: 'bid' | 'ask'
  price: string
  amountHuman: string
}
