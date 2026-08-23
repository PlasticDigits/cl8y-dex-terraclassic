import { beforeEach, describe, expect, it, vi } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import { renderWithProviders } from '@/test-utils'
import { useWalletStore } from '@/hooks/useWallet'
import { useDexStore } from '@/stores/dex'
import { DEFAULT_SLIPPAGE_TOLERANCE_PERCENT } from '@/utils/slippageProtectionCopy'
import type { Invoice, PayInvoiceQuote } from '@/utils/payInvoice'
import { PayWithAnyToken } from './PayWithAnyToken'

const UST1 = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const PAYEE = 'terra1xsecn4snv94ezcez0z3vq8an9j4h4kxxcydp8l'
const WALLET = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
const HOOK = btoa(JSON.stringify({ enable_feature: { sku: 'transfer_tax' } }))

const INVOICE: Invoice = {
  invoiceToken: UST1,
  invoiceAmount: '50000000',
  payee: PAYEE,
  hookMsg: HOOK,
}

const DIRECT_OK: PayInvoiceQuote = {
  status: 'ok',
  kind: 'direct',
  payToken: UST1,
  offerCw20: UST1,
  payRaw: '50000000',
  cw20SendAmount: '50000000',
  maxIn: '50000000',
  minInvoiceOut: '50000000',
  operations: [],
  routeLabel: '',
  hopCount: 0,
}

const quotePayInvoice = vi.fn(async (): Promise<PayInvoiceQuote> => DIRECT_OK)
const buildPayInvoiceMsgs = vi.fn(() => [
  { contract: UST1, msg: { send: { contract: PAYEE, amount: '50000000', msg: HOOK } } },
])

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn(), playSuccess: vi.fn(), playError: vi.fn() },
}))

vi.mock('@/services/terraclassic/wrapMapper', () => ({
  queryWrapMapperConfig: vi.fn(async () => null),
  wrapMapperFeeBps: () => null,
}))

vi.mock('@/hooks/useTokenBalance', () => ({
  useTokenBalance: () => ({ data: '50000000', isLoading: false }),
}))

vi.mock('@/hooks/useNativeUlunaBalance', () => ({
  useNativeUlunaBalance: () => ({ data: '1000000' }),
}))

vi.mock('@/utils/payInvoice', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/payInvoice')>()
  return {
    ...actual,
    quotePayInvoice: (...args: unknown[]) => quotePayInvoice(...(args as [])),
    buildPayInvoiceMsgs: (...args: unknown[]) => buildPayInvoiceMsgs(...(args as [])),
  }
})

const INVOICE_PROP: Invoice = { ...INVOICE }

describe('PayWithAnyToken (GitLab #595)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    quotePayInvoice.mockResolvedValue(DIRECT_OK)
    useWalletStore.setState({ address: null, walletType: null, error: null })
    useDexStore.setState({ slippageTolerance: DEFAULT_SLIPPAGE_TOLERANCE_PERCENT })
  })

  it('renders disconnected with Pay gated and invoice copy (not Swap CTA)', async () => {
    renderWithProviders(<PayWithAnyToken invoice={INVOICE_PROP} tokens={[UST1]} invoiceSymbol="UST1" />, {
      route: '/token/create?payee=terra1spoofed',
    })
    expect(screen.getByTestId('pay-with-any-token')).toBeInTheDocument()
    expect(screen.getByTestId('pay-invoice-connect')).toHaveTextContent(/connect wallet/i)
    expect(screen.queryByTestId('pay-invoice-cta')).not.toBeInTheDocument()
    expect(screen.getByTestId('pay-invoice-amount')).toHaveTextContent('50')
    expect(screen.getByTestId('pay-invoice-slippage-presets')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^swap$/i })).not.toBeInTheDocument()
    await waitFor(() => expect(quotePayInvoice).toHaveBeenCalled())
    const passed = quotePayInvoice.mock.calls[0]?.[0] as { invoice: Invoice }
    expect(passed.invoice.payee).toBe(PAYEE)
  })

  it('connected: Pay CTA, 5% slippage default, no payee from query string', async () => {
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    renderWithProviders(<PayWithAnyToken invoice={INVOICE_PROP} tokens={[UST1]} cta="pay" />, {
      route: '/manage?payee=terra1spoofed&treasury=terra1spoofed',
    })
    const cta = await screen.findByTestId('pay-invoice-cta')
    expect(cta).toHaveTextContent('Pay')
    expect(screen.getByTestId('pay-invoice-slippage-preset-5')).toHaveClass('tab-glass-active')
    await waitFor(() => expect(buildPayInvoiceMsgs).toHaveBeenCalled())
    const built = buildPayInvoiceMsgs.mock.calls[0]?.[0] as { invoice: Invoice }
    expect(built.invoice.payee).toBe(PAYEE)
  })

  it('Enable CTA label and No route disables Pay', async () => {
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    quotePayInvoice.mockResolvedValue({ status: 'unavailable', disableReason: 'No route' })
    renderWithProviders(<PayWithAnyToken invoice={INVOICE_PROP} tokens={[UST1]} cta="enable" />)
    expect(await screen.findByTestId('pay-invoice-disable')).toHaveTextContent('No route')
    expect(screen.getByTestId('pay-invoice-cta')).toBeDisabled()
    expect(screen.getByTestId('pay-invoice-cta')).toHaveTextContent('Enable')
  })
})
