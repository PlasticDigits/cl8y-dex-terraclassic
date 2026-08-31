import { render, screen } from '@testing-library/react'
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom'
import { describe, expect, it } from 'vitest'
import { SwapAliasRedirect } from '../SwapAliasRedirect'

function LocationEcho() {
  const { pathname, search, hash } = useLocation()
  return (
    <div>
      <div data-testid="pathname">{pathname}</div>
      <div data-testid="search">{search}</div>
      <div data-testid="hash">{hash}</div>
    </div>
  )
}

function renderAlias(entry: string) {
  return render(
    <MemoryRouter initialEntries={[entry]}>
      <Routes>
        <Route path="/" element={<LocationEcho />} />
        <Route path="/swap" element={<SwapAliasRedirect />} />
        <Route path="/swap/" element={<SwapAliasRedirect />} />
        <Route path="*" element={<LocationEcho />} />
      </Routes>
    </MemoryRouter>
  )
}

describe('SwapAliasRedirect (#711)', () => {
  it('preserves search and hash when redirecting /swap to /', () => {
    renderAlias('/swap?from=uluna&to=uusd#junk')
    expect(screen.getByTestId('pathname')).toHaveTextContent('/')
    expect(screen.getByTestId('search')).toHaveTextContent('?from=uluna&to=uusd')
    expect(screen.getByTestId('hash')).toHaveTextContent('#junk')
  })

  it('preserves search on /swap/ trailing slash', () => {
    renderAlias('/swap/?inputCurrency=uluna&outputCurrency=uusd')
    expect(screen.getByTestId('pathname')).toHaveTextContent('/')
    expect(screen.getByTestId('search')).toHaveTextContent('?inputCurrency=uluna&outputCurrency=uusd')
  })
})
