import { test, expect } from './fixtures/dev-wallet'

test.describe('Create Pair Page', () => {
  test('shows Create Trading Pair heading', async ({ page }) => {
    await page.goto('/create')
    await expect(page.getByRole('heading', { name: /Create Trading Pair/i })).toBeVisible()
  })

  test('has two token address input fields', async ({ page }) => {
    await page.goto('/create')
    const inputs = page.getByPlaceholder('terra1...')
    await expect(inputs).toHaveCount(2)
  })

  test('shows labels for Token A and Token B', async ({ page }) => {
    await page.goto('/create')
    await expect(page.getByText(/Token A/i)).toBeVisible()
    await expect(page.getByText(/Token B/i)).toBeVisible()
  })

  test('shows prerequisites info box', async ({ page }) => {
    await page.goto('/create')
    await expect(page.getByText(/Before creating a pair/i)).toBeVisible()
    await expect(page.getByText(/valid CW20 contracts/i)).toBeVisible()
    await expect(page.getByText(/whitelisted by governance/i)).toBeVisible()
    await expect(page.getByText(/must not already exist/i)).toBeVisible()
  })

  test('shows Connect Wallet button when disconnected', async ({ page }) => {
    await page.goto('/create')
    const submitBtn = page.getByRole('button', { name: /Connect Wallet/i }).last()
    await expect(submitBtn).toBeVisible()
    await expect(submitBtn).toBeDisabled()
  })

  test('accepts terra address input in Token A field', async ({ page }) => {
    await page.goto('/create')
    const tokenAInput = page.getByPlaceholder('terra1...').first()
    const testAddr = 'terra1yyca08xqdgvjz0psg56z67ejh9xms6l436u8y58m82npdqqhmmtqzjqhh0'
    await tokenAInput.fill(testAddr)
    await expect(tokenAInput).toHaveValue(testAddr)
  })

  test('accepts terra address input in Token B field', async ({ page }) => {
    await page.goto('/create')
    const tokenBInput = page.getByPlaceholder('terra1...').last()
    const testAddr = 'terra1yw4xvtc43me9scqfr2jr2gzvcxd3a9y4eq7gaukreugw2yd2f8tsrnr34u'
    await tokenBInput.fill(testAddr)
    await expect(tokenBInput).toHaveValue(testAddr)
  })

  test.describe('With wallet connected', () => {
    test('shows Create Pair button when connected', async ({ page, connectWallet }) => {
      await connectWallet
      await page.goto('/create')

      const createBtn = page.getByRole('button', { name: /Create Pair/i })
      await expect(createBtn).toBeVisible()
    })

    test('Create Pair button is disabled without both addresses', async ({ page, connectWallet }) => {
      await connectWallet
      await page.goto('/create')

      const createBtn = page.getByRole('button', { name: /Create Pair/i })
      await expect(createBtn).toBeDisabled()
    })

    test('can fill both addresses and see enabled button', async ({ page, connectWallet }) => {
      await connectWallet
      await page.goto('/create')

      const tokenAInput = page.getByPlaceholder('terra1...').first()
      const tokenBInput = page.getByPlaceholder('terra1...').last()

      await tokenAInput.fill('terra1yyca08xqdgvjz0psg56z67ejh9xms6l436u8y58m82npdqqhmmtqzjqhh0')
      await tokenBInput.fill('terra1yw4xvtc43me9scqfr2jr2gzvcxd3a9y4eq7gaukreugw2yd2f8tsrnr34u')

      const createBtn = page.getByRole('button', { name: /Create Pair/i })
      await expect(createBtn).toBeEnabled()
    })
  })
})
