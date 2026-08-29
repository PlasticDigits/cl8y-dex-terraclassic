import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { LimitOrderPriceInputWithContext } from '../LimitOrderPriceField'
import { LIMIT_PRICE_DEVIATION_CHIP_PRESETS, limitPriceFromRefDeviationChip } from '@/utils/limitOrderPriceReference'

describe('LimitOrderPriceInputWithContext', () => {
  it('renders ask chips above ref and sets a valid sell price (#495)', async () => {
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
    expect(screen.getByTestId('limit-order-price-chip-0')).toHaveTextContent('0%+')
    expect(screen.getByTestId('limit-order-price-chip-1')).toHaveTextContent('+1%')

    await user.click(screen.getByTestId('limit-order-price-chip-5'))
    expect(onPriceChange).toHaveBeenCalledWith(limitPriceFromRefDeviationChip('ask', 3, 5))
    expect(onPriceChange).toHaveBeenCalledWith('3.15')
  })

  it('renders bid chips below ref and clears Invalid buy for every preset (#495)', async () => {
    const user = userEvent.setup()
    const ref = 3
    let price = '3'
    const onPriceChange = vi.fn((v: string) => {
      price = v
    })

    const { rerender } = render(
      <LimitOrderPriceInputWithContext
        side="bid"
        price={price}
        onPriceChange={onPriceChange}
        inputId="price"
        refToken1PerToken0={ref}
        refSource="tape"
        tapeHeadlineUsd="1"
        token0Label="AAA"
        token1Label="BBB"
      />
    )

    expect(screen.getByTestId('limit-order-price-chip-0')).toHaveTextContent('0%−')
    expect(screen.getByTestId('limit-order-price-chip-1')).toHaveTextContent('−1%')
    expect(screen.getByText('Invalid buy')).toBeInTheDocument()

    for (const mag of LIMIT_PRICE_DEVIATION_CHIP_PRESETS) {
      await user.click(screen.getByTestId(`limit-order-price-chip-${mag}`))
      expect(onPriceChange).toHaveBeenLastCalledWith(limitPriceFromRefDeviationChip('bid', ref, mag))
      rerender(
        <LimitOrderPriceInputWithContext
          side="bid"
          price={price}
          onPriceChange={onPriceChange}
          inputId="price"
          refToken1PerToken0={ref}
          refSource="tape"
          tapeHeadlineUsd="1"
          token0Label="AAA"
          token1Label="BBB"
        />
      )
      expect(screen.queryByText('Invalid buy')).not.toBeInTheDocument()
    }
  })

  it('ask near-market 0%+ clears Invalid sell (#495)', async () => {
    const user = userEvent.setup()
    let price = '3'
    const onPriceChange = vi.fn((v: string) => {
      price = v
    })

    const { rerender } = render(
      <LimitOrderPriceInputWithContext
        side="ask"
        price={price}
        onPriceChange={onPriceChange}
        inputId="price"
        refToken1PerToken0={3}
        refSource="tape"
        tapeHeadlineUsd="1"
        token0Label="AAA"
        token1Label="BBB"
      />
    )

    expect(screen.getByText('Invalid sell')).toBeInTheDocument()

    await user.click(screen.getByTestId('limit-order-price-chip-0'))
    expect(onPriceChange).toHaveBeenCalledWith(limitPriceFromRefDeviationChip('ask', 3, 0))
    rerender(
      <LimitOrderPriceInputWithContext
        side="ask"
        price={price}
        onPriceChange={onPriceChange}
        inputId="price"
        refToken1PerToken0={3}
        refSource="tape"
        tapeHeadlineUsd="1"
        token0Label="AAA"
        token1Label="BBB"
      />
    )
    expect(screen.queryByText('Invalid sell')).not.toBeInTheDocument()
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

  it('hides deviation chips when showDeviationChrome is false (#693)', () => {
    render(
      <LimitOrderPriceInputWithContext
        side="ask"
        price="3"
        onPriceChange={vi.fn()}
        inputId="price"
        refToken1PerToken0={3}
        refSource="tape"
        tapeHeadlineUsd="1"
        token0Label="AAA"
        token1Label="BBB"
        showDeviationChrome={false}
      />
    )
    expect(screen.queryByTestId('limit-order-price-deviation-chips')).not.toBeInTheDocument()
    expect(screen.getByTestId('limit-order-price-input')).toBeInTheDocument()
    expect(screen.getByText('Invalid sell')).toBeInTheDocument()
  })
})
