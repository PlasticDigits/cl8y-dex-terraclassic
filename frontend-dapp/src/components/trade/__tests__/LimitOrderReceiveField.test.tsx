import { act, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { writeNumberDisplayMode } from '@/utils/numberDisplayPreference'
import { LimitOrderReceiveField } from '../LimitOrderReceiveField'

beforeEach(() => localStorage.clear())
afterEach(() => localStorage.clear())

describe('LimitOrderReceiveField number display (#1259)', () => {
  it('reactively formats receive chrome and balance from unchanged plain amounts', () => {
    render(
      <LimitOrderReceiveField
        receiveLabel="USTR"
        receiveAmountHuman="1234567"
        receiveDecimals={6}
        receiveBalanceQuery={{ data: '1234567000000', isLoading: false, isError: false } as never}
        walletConnected
      />
    )
    const amount = screen.getByTestId('limit-order-receive-amount')
    expect(amount.tagName).toBe('DIV')
    expect(amount).toHaveTextContent('1.23457M')
    expect(screen.getByTestId('limit-order-receive-balance')).toHaveTextContent('1.235M')

    act(() => writeNumberDisplayMode('full'))
    expect(amount).toHaveTextContent('1,234,567')
    expect(screen.getByTestId('limit-order-receive-balance')).toHaveTextContent('1,234,567')

    act(() => writeNumberDisplayMode('compact'))
    expect(amount).toHaveTextContent('1.23457M')
  })
})
