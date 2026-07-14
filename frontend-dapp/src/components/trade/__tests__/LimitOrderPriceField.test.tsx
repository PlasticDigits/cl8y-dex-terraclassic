import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { LimitOrderPriceInputWithContext } from '../LimitOrderPriceField'

describe('LimitOrderPriceInputWithContext', () => {
  it('renders deviation chips and sets price from reference (#488)', async () => {
    const user = userEvent.setup()
    const onPriceChange = vi.fn()

    render(
      <LimitOrderPriceInputWithContext
        side="ask"
        price="3"
        onPriceChange={onPriceChange}
        inputId="price"
        refToken1PerToken0={3}
        refSource="tape"
        tapeHeadlineUsd="1"
        token0Label="AAA"
        token1Label="BBB"
      />
    )

    expect(screen.getByTestId('limit-order-price-deviation-chips')).toBeInTheDocument()
    expect(screen.getByTestId('limit-order-price-chip-1')).toHaveTextContent('+1%')

    await user.click(screen.getByTestId('limit-order-price-chip-5'))
    expect(onPriceChange).toHaveBeenCalledWith('3.15')
  })

  it('disables chips when reference is unavailable', () => {
    render(
      <LimitOrderPriceInputWithContext
        side="bid"
        price="1"
        onPriceChange={vi.fn()}
        inputId="price"
        refToken1PerToken0={null}
        refSource={null}
        tapeHeadlineUsd={null}
        token0Label="AAA"
        token1Label="BBB"
      />
    )

    expect(screen.getByTestId('limit-order-price-chip-0')).toBeDisabled()
    expect(screen.getByTestId('limit-order-price-chip-10')).toBeDisabled()
  })
})
