import { Outlet, NavLink } from 'react-router-dom'
import { useEffect, useState } from 'react'
import WalletButton from '@/components/wallet/WalletButton'
import NetworkBadge from '@/components/wallet/NetworkBadge'
import { sounds } from '@/lib/sounds'

type ThemeMode = 'dark' | 'light'

const NAV_ITEMS = [
  { path: '/', label: 'Swap', end: true, icon: '/assets/icon-swap.png' },
  { path: '/pool', label: 'Pool', icon: '/assets/icon-pool.png' },
  { path: '/charts', label: 'Charts', icon: '/assets/icon-chart.png' },
  { path: '/trader', label: 'Trader', end: false, icon: '/assets/icon-price-up.png' },
  { path: '/protocol', label: 'Protocol', icon: '/assets/icon-settings.png' },
  { path: '/tiers', label: 'Fee Tiers', icon: '/assets/icon-fee-tiers.png' },
  { path: '/create', label: 'Create Pair', icon: '/assets/icon-create-pair.png' },
]

function getSystemTheme(): ThemeMode {
  if (typeof window === 'undefined') return 'dark'
  return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark'
}

function getInitialTheme(): ThemeMode {
  const stored = window.localStorage.getItem('cl8y-dex-theme')
  if (stored === 'dark' || stored === 'light') return stored
  return getSystemTheme()
}

export default function Layout() {
  const [theme, setTheme] = useState<ThemeMode>(getInitialTheme)

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme)
  }, [theme])

  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: light)')
    const handler = () => {
      if (!window.localStorage.getItem('cl8y-dex-theme')) {
        setTheme(media.matches ? 'light' : 'dark')
      }
    }
    media.addEventListener('change', handler)
    return () => media.removeEventListener('change', handler)
  }, [])

  const setThemeAndPersist = (mode: ThemeMode) => {
    setTheme(mode)
    window.localStorage.setItem('cl8y-dex-theme', mode)
  }

  return (
    <div className="min-h-screen overflow-x-hidden">
      <header
        className="sticky top-0 z-30 border-b-2 border-white/40 overflow-x-clip"
        style={{ background: 'var(--panel-bg-strong)', backdropFilter: 'blur(8px)' }}
      >
        <div className="max-w-7xl mx-auto px-3 sm:px-6 lg:px-8">
          <div className="relative flex flex-col md:flex-row md:items-center md:justify-between min-h-14 py-2 gap-2">
            <NavLink
              to="/"
              className="flex w-fit items-center justify-center md:justify-start order-first md:order-1 shrink-0 py-1 md:py-1.5 px-1.5 md:px-4 rounded-sm border-2 border-black shadow-[3px_3px_0_#000] hover:translate-x-[1px] hover:translate-y-[1px] hover:shadow-[2px_2px_0_#000] pointer-events-auto transition-all duration-150"
              style={{ background: 'var(--header-logo-bg)' }}
              onClick={() => sounds.playButtonPress()}
            >
              <img
                src="/assets/cl8y-dex-header-logo.png"
                alt="CL8Y DEX"
                className="h-7 sm:h-12 md:h-16 w-auto max-w-[512px] shrink-0 rounded-none object-contain object-center"
              />
            </NavLink>

            <nav
              className="flex gap-1 border-2 border-white/30 p-1 min-w-0 w-full md:w-auto md:flex-1 order-2 overflow-x-auto"
              style={{ background: 'var(--panel-bg)' }}
            >
              {NAV_ITEMS.map(({ path, label, end, icon }) => (
                <NavLink
                  key={path}
                  to={path}
                  end={end !== false}
                  aria-label={label}
                  title={label}
                  onClick={() => sounds.playButtonPress()}
                  className={({ isActive }) =>
                    `flex-shrink-0 flex items-center justify-center gap-1.5 px-2 lg:px-3.5 py-2 text-[10px] lg:text-xs font-medium whitespace-nowrap uppercase tracking-[0.04em] lg:tracking-wide border transition-colors ${
                      isActive
                        ? 'bg-[#202614] text-[#d5ff7f] border-[#b8ff3d]/60 shadow-[2px_2px_0_#000]'
                        : 'text-slate-200 border-transparent hover:border-white/40 hover:bg-zinc-800'
                    }`
                  }
                >
                  <img src={icon} alt="" className="h-4 w-auto shrink-0 -mt-0.5" aria-hidden />
                  <span className="hidden lg:inline">{label}</span>
                </NavLink>
              ))}
            </nav>

            <div className="flex w-full items-center justify-between gap-2 shrink-0 order-3 sm:w-auto sm:justify-start">
              <NetworkBadge />
              <WalletButton />
            </div>
          </div>
        </div>
      </header>

      <main className="relative max-w-5xl mx-auto px-3 sm:px-4 pt-2.5 pb-5 md:pt-4 md:pb-8">
        <div
          aria-hidden="true"
          className="pointer-events-none absolute inset-x-0 top-2 mx-auto h-[520px] max-w-3xl rounded-[40px] theme-hero-glow blur-3xl"
        />
        <div className="relative z-10">
          <Outlet />
        </div>
      </main>

      <footer
        className="border-t-2 border-white/25 py-4 md:py-6 text-xs md:text-sm uppercase tracking-wider"
        style={{ color: 'var(--ink-dim)' }}
      >
        <div className="mx-auto max-w-5xl px-4 flex flex-col gap-3 items-center justify-center md:flex-row md:justify-between">
          <p>CL8Y DEX · Terra Classic</p>
          <div className="flex items-center gap-2" role="group" aria-label="Theme">
            <div
              className="inline-flex border border-white/50 p-0.5 rounded-sm"
              style={{ background: 'var(--panel-bg)' }}
            >
              <button
                type="button"
                aria-pressed={theme === 'dark'}
                aria-label="Dark theme"
                className={`px-2.5 py-1 text-[11px] md:text-xs uppercase tracking-wider transition-colors ${
                  theme === 'dark' ? 'bg-white/20 text-inherit' : 'text-slate-400 hover:text-slate-300'
                }`}
                onClick={() => {
                  sounds.playButtonPress()
                  setThemeAndPersist('dark')
                }}
              >
                Dark
              </button>
              <button
                type="button"
                aria-pressed={theme === 'light'}
                aria-label="Light theme"
                className={`px-2.5 py-1 text-[11px] md:text-xs uppercase tracking-wider transition-colors ${
                  theme === 'light' ? 'bg-white/20 text-inherit' : 'text-slate-400 hover:text-slate-300'
                }`}
                onClick={() => {
                  sounds.playButtonPress()
                  setThemeAndPersist('light')
                }}
              >
                Light
              </button>
            </div>
          </div>
        </div>
      </footer>
    </div>
  )
}
