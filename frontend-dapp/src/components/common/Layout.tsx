import { Outlet, NavLink, useLocation } from 'react-router-dom'
import { useEffect, useMemo, useRef, useState } from 'react'
import WalletButton from '@/components/wallet/WalletButton'
import NetworkBadge from '@/components/wallet/NetworkBadge'
import { ThemeSegmentedControl, type ThemeMode } from '@/components/common/ThemeSegmentedControl'
import { MORE_NAV_ITEMS, PRIMARY_NAV_ITEMS } from '@/components/common/navItems'
import { useSyncMobileNavStack } from '@/hooks/useSyncMobileNavStack'
import { sounds } from '@/lib/sounds'

function getInitialTheme(): ThemeMode {
  if (typeof window === 'undefined') return 'dark'
  const stored = window.localStorage.getItem('cl8y-dex-theme')
  if (stored === 'dark' || stored === 'light') return stored
  return 'dark'
}

export default function Layout() {
  const location = useLocation()
  const mobileNavRef = useRef<HTMLElement>(null)
  useSyncMobileNavStack(mobileNavRef)

  const [theme, setTheme] = useState<ThemeMode>(getInitialTheme)
  const [isMoreMenuOpen, setIsMoreMenuOpen] = useState(false)
  const [isMobileMoreOpen, setIsMobileMoreOpen] = useState(false)

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme)
  }, [theme])

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setIsMoreMenuOpen(false)
        setIsMobileMoreOpen(false)
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [])

  const setThemeAndPersist = (mode: ThemeMode) => {
    sounds.playButtonPress()
    setTheme(mode)
    window.localStorage.setItem('cl8y-dex-theme', mode)
  }

  useEffect(() => {
    setIsMoreMenuOpen(false)
    setIsMobileMoreOpen(false)
  }, [location.pathname])

  const isMoreRoute = useMemo(
    () =>
      MORE_NAV_ITEMS.some((item) =>
        item.end === false ? location.pathname.startsWith(item.path) : location.pathname === item.path
      ),
    [location.pathname]
  )

  return (
    <div className="app-shell">
      {(isMoreMenuOpen || isMobileMoreOpen) && (
        <button
          type="button"
          aria-label="Close navigation menu"
          className="app-menu-dismiss"
          onClick={() => {
            setIsMoreMenuOpen(false)
            setIsMobileMoreOpen(false)
          }}
        />
      )}

      <header className="app-header-shell">
        <div className="app-header">
          <NavLink
            to="/"
            className="app-brand"
            onClick={() => {
              sounds.playButtonPress()
            }}
          >
            <span className="app-brand-mark">
              <img src="/assets/cl8y-dex-glass-logo.svg" alt="CL8Y DEX" className="app-brand-logo" />
            </span>
            <span className="app-brand-copy">
              <span className="app-brand-kicker">Terra Classic ecosystem</span>
              <strong className="app-brand-title">CL8Y DEX</strong>
            </span>
          </NavLink>

          <nav className="app-desktop-nav" aria-label="Primary">
            {PRIMARY_NAV_ITEMS.map((item) => (
              <NavLink
                key={item.path}
                to={item.path}
                end={item.end !== false}
                onClick={() => {
                  sounds.playButtonPress()
                }}
                className={({ isActive }) => `app-nav-link${isActive ? ' app-nav-link-active' : ''}`}
              >
                <span className="app-nav-link-label">{item.label}</span>
              </NavLink>
            ))}

            <div className="app-more-wrap">
              <button
                type="button"
                className={`app-more-trigger${isMoreRoute ? ' app-nav-link-active' : ''}`}
                aria-haspopup="menu"
                aria-expanded={isMoreMenuOpen}
                onClick={() => {
                  sounds.playButtonPress()
                  setIsMoreMenuOpen((current) => !current)
                }}
              >
                <span className="app-nav-link-label">More</span>
                <span aria-hidden="true" className="text-xs">
                  {isMoreMenuOpen ? '▲' : '▼'}
                </span>
              </button>

              {isMoreMenuOpen && (
                <div role="menu" className="app-menu">
                  {MORE_NAV_ITEMS.map((item) => (
                    <NavLink
                      key={item.path}
                      to={item.path}
                      end={item.end !== false}
                      role="menuitem"
                      onClick={() => {
                        sounds.playButtonPress()
                        setIsMoreMenuOpen(false)
                      }}
                      className={({ isActive }) => `app-menu-link${isActive ? ' app-nav-link-active' : ''}`}
                    >
                      {item.label}
                    </NavLink>
                  ))}
                </div>
              )}
            </div>
          </nav>

          <div className="app-header-controls">
            <NetworkBadge />
            <WalletButton />
          </div>
        </div>
      </header>

      <main className="app-main-shell">
        <div className="app-main">
          <div aria-hidden="true" className="app-hero-glow" />
          <div className="app-main-content">
            <Outlet />
          </div>
        </div>
      </main>

      <footer className="app-footer-shell">
        <div className="app-footer">
          <p>CL8Y DEX · Terra Classic</p>
          <ThemeSegmentedControl
            theme={theme}
            onSelect={setThemeAndPersist}
            groupClassName="app-footer-theme-group"
            labelStyle="short"
          />
        </div>
      </footer>

      <nav ref={mobileNavRef} className="app-mobile-nav-shell" aria-label="Primary">
        {PRIMARY_NAV_ITEMS.map((item) => (
          <NavLink
            key={item.path}
            to={item.path}
            end={item.end !== false}
            onClick={() => {
              sounds.playButtonPress()
            }}
            className={({ isActive }) => `app-mobile-link${isActive ? ' app-mobile-link-active' : ''}`}
          >
            {item.label}
          </NavLink>
        ))}
        <button
          type="button"
          className={`app-mobile-more${isMoreRoute ? ' app-mobile-more-active' : ''}`}
          aria-haspopup="menu"
          aria-expanded={isMobileMoreOpen}
          onClick={() => {
            sounds.playButtonPress()
            setIsMobileMoreOpen((current) => !current)
          }}
        >
          More
        </button>
      </nav>

      {isMobileMoreOpen && (
        <section className="app-mobile-more-sheet" aria-label="More pages">
          {MORE_NAV_ITEMS.map((item) => (
            <NavLink
              key={item.path}
              to={item.path}
              end={item.end !== false}
              onClick={() => {
                sounds.playButtonPress()
                setIsMobileMoreOpen(false)
              }}
              className={({ isActive }) => `app-menu-link${isActive ? ' app-nav-link-active' : ''}`}
            >
              {item.label}
            </NavLink>
          ))}
          <ThemeSegmentedControl
            theme={theme}
            onSelect={setThemeAndPersist}
            groupClassName="app-mobile-theme-group"
            labelStyle="long"
          />
        </section>
      )}
    </div>
  )
}
