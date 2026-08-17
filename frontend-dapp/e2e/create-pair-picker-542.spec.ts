import { test, expect } from './fixtures/dev-wallet'
import {
  ARIA_SELECT_TOKEN_A,
  ARIA_SELECT_TOKEN_B,
  createPairTokenA,
  fillCreatePairCustom,
  openCreatePairCustom,
  pickCreatePairToken,
  pickCreatePairTokenByAddress,
  readFrontendEnvLocal,
} from './helpers/create-pair-picker'

const MAINNET_CLUNC = 'terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg'

test.describe('Create Pair listed-CW20 picker (GitLab #542)', () => {
  test('P1: pick two listed/env CW20s — addresses match .env.local when set', async ({ page }) => {
    const env = readFrontendEnvLocal()
    await page.goto('/create')
    await expect(createPairTokenA(page)).toBeEnabled({ timeout: 20_000 })

    const localClunc = env.VITE_LUNC_C_TOKEN_ADDRESS?.trim()
    const localCl8y = env.VITE_CL8Y_TOKEN_ADDRESS?.trim()
    const pickedA = localClunc
      ? await pickCreatePairTokenByAddress(page, ARIA_SELECT_TOKEN_A, localClunc)
      : await pickCreatePairToken(page, ARIA_SELECT_TOKEN_A, 'cLUNC')
    const pickedB = localCl8y
      ? await pickCreatePairTokenByAddress(page, ARIA_SELECT_TOKEN_B, localCl8y)
      : await pickCreatePairToken(page, ARIA_SELECT_TOKEN_B, 'CL8Y')
    expect(pickedA, 'cLUNC / wrap env token should be in the Create Pair catalog').toBe(true)
    expect(pickedB, 'CL8Y / env token should be in the Create Pair catalog').toBe(true)

    await openCreatePairCustom(page, 'a')
    const addrA = await page.getByLabel(/Token A Contract Address/i).inputValue()
    expect(addrA.startsWith('terra1')).toBe(true)

    if (localClunc) {
      expect(addrA.toLowerCase()).toBe(localClunc.toLowerCase())
      expect(addrA.toLowerCase()).not.toBe(MAINNET_CLUNC)
    }

    await openCreatePairCustom(page, 'b')
    const addrB = await page.getByLabel(/Token B Contract Address/i).inputValue()
    if (localCl8y) {
      expect(addrB.toLowerCase()).toBe(localCl8y.toLowerCase())
    }
  })

  test('P2: custom paste of a LocalTerra gem / unlisted CW20', async ({ page }) => {
    const env = readFrontendEnvLocal()
    const gem =
      env.VITE_TOKEN_EMBER_ADDRESS?.trim() || 'terra1yyca08xqdgvjz0psg56z67ejh9xms6l436u8y58m82npdqqhmmtqzjqhh0'
    const other =
      env.VITE_TOKEN_CORAL_ADDRESS?.trim() || 'terra1yw4xvtc43me9scqfr2jr2gzvcxd3a9y4eq7gaukreugw2yd2f8tsrnr34u'

    await page.goto('/create')
    await fillCreatePairCustom(page, 'a', gem)
    await fillCreatePairCustom(page, 'b', other)
    await expect(page.getByLabel(/Token A Contract Address/i)).toHaveValue(gem)
    await expect(page.getByLabel(/Token B Contract Address/i)).toHaveValue(other)
  })

  test('P3: picking two tokens that already have a pair does not show a false success', async ({ page }) => {
    await page.goto('/create')
    await expect(createPairTokenA(page)).toBeEnabled({ timeout: 20_000 })

    const pickedA = await pickCreatePairToken(page, ARIA_SELECT_TOKEN_A, 'cLUNC')
    const pickedB = await pickCreatePairToken(page, ARIA_SELECT_TOKEN_B, 'cUSTC')
    test.skip(!pickedA || !pickedB, 'cLUNC/cUSTC not in catalog on this env')

    await expect(page.getByText(/Pair Created Successfully/i)).toHaveCount(0)
    const createBtn = page.getByRole('button', { name: /Create Pair|Connect Wallet/i }).last()
    await expect(createBtn).toBeVisible()
    if (await createBtn.isEnabled()) {
      await createBtn.click()
      await expect(page.getByText(/Pair Created Successfully/i)).toHaveCount(0)
    }
  })

  test('P4: combobox keyboard — Arrow/Enter/Escape', async ({ page }) => {
    await page.goto('/create')
    const combo = createPairTokenA(page)
    await expect(combo).toBeEnabled({ timeout: 20_000 })
    await combo.focus()
    await page.keyboard.press('ArrowDown')
    const list = page.getByRole('listbox', { name: ARIA_SELECT_TOKEN_A })
    await expect(list).toBeVisible()
    await page.keyboard.press('Escape')
    await expect(list).toHaveCount(0)

    await combo.focus()
    await page.keyboard.press('ArrowDown')
    await expect(list).toBeVisible()
    await page.keyboard.press('Enter')
    await expect(list).toHaveCount(0)
    await expect(combo).not.toHaveValue('')
  })

  test('P5: phone width (~390px) list is usable without clipping the trigger', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await page.goto('/create')
    const combo = createPairTokenA(page)
    await expect(combo).toBeEnabled({ timeout: 20_000 })

    const before = await combo.boundingBox()
    expect(before).toBeTruthy()
    await combo.click()
    const list = page.locator('.token-select-dropdown')
    await expect(list).toBeVisible()
    const listBox = await list.boundingBox()
    expect(listBox).toBeTruthy()
    expect(listBox!.width).toBeGreaterThan(40)
    expect(listBox!.height).toBeGreaterThan(40)

    const open = await combo.boundingBox()
    expect(Math.abs((open?.y ?? 0) - (before?.y ?? 0))).toBeLessThan(2)
    await page.keyboard.press('Escape')
  })

  test('C2: natives are absent from the picker list', async ({ page }) => {
    await page.goto('/create')
    await createPairTokenA(page).click()
    const list = page.getByRole('listbox', { name: ARIA_SELECT_TOKEN_A })
    await expect(list).toBeVisible()
    const texts = await list.getByRole('option').allTextContents()
    expect(texts.some((t) => t.includes('uluna') || t.includes('uusd'))).toBe(false)
    expect(await list.getByTestId('token-option-uluna').count()).toBe(0)
  })
})
