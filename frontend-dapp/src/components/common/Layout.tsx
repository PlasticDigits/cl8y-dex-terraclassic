import { Outlet, NavLink, useLocation } from 'react-router-dom'
import { useEffect, useMemo, useRef, useState } from 'react'
import WalletButton from '@/components/wallet/WalletButton'
import NetworkBadge from '@/components/wallet/NetworkBadge'
import EnvironmentRibbon from '@/components/legal/EnvironmentRibbon'
import LegalFooterNotice from '@/components/legal/LegalFooterNotice'
import RiskAcknowledgementModal from '@/components/legal/RiskAcknowledgementModal'
import { ThemeSegmentedControl, type ThemeMode } from '@/components/common/ThemeSegmentedControl'
import {
  getHeaderMoreMenuItems,
  HEADER_FULL_NAV_MIN_WIDTH_PX,
  MORE_NAV_ITEMS,
  PRIMARY_NAV_ITEMS,
} from '@/components/common/navItems'
import { useMediaQuery } from '@/hooks/useMediaQuery'
import { useSyncMobileNavStack } from '@/hooks/useSyncMobileNavStack'
import { sounds } from '@/lib/sounds'
import { LcdConnectivityBanner } from '@/components/common/LcdConnectivityBanner'
import { useLcdConnectivityRecovery } from '@/hooks/useLcdConnectivityRecovery'
import { RouteContentReadyProvider } from '@/contexts/RouteContentReadyContext'

function getInitialTheme(): ThemeMode {
  if (typeof window === 'undefined') return 'dark'
  const stored = window.localStorage.getItem('cl8y-dex-theme')
  if (stored === 'dark' || stored === 'light') return stored
  return 'dark'
}

export default function Layout() {
  const { isLcdUnreachable, isProbePending, retryAll } = useLcdConnectivityRecovery()
  const location = useLocation()
  const mobileNavRef = useRef<HTMLElement>(null)
  useSyncMobileNavStack(mobileNavRef)

  const fullDesktopHeader = useMediaQuery(`(min-width: ${HEADER_FULL_NAV_MIN_WIDTH_PX}px)`)
  const showMobileLegalStrip = useMediaQuery('(max-width: 767px)')

  const [theme, setTheme] = useState<ThemeMode>(getInitialTheme)
  const [isMoreMenuOpen, setIsMoreMenuOpen] = useState(false)
  const [isMobileMoreOpen, setIsMobileMoreOpen] = useState(false)
  /** Defer legal notice until route page mounts so it is not the LCP element (GitLab #179). */
  const [routeContentReady, setRouteContentReady] = useState(false)

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

  const headerMoreMenuItems = useMemo(() => getHeaderMoreMenuItems(fullDesktopHeader), [fullDesktopHeader])

  const isHeaderMoreRouteActive = useMemo(
    () =>
      headerMoreMenuItems.some((item) =>
        item.end === false ? location.pathname.startsWith(item.path) : location.pathname === item.path
      ),
    [location.pathname, headerMoreMenuItems]
  )

  const isMobileMoreSheetRoute = useMemo(
    () =>
      MORE_NAV_ITEMS.some((item) =>
        item.end === false ? location.pathname.startsWith(item.path) : location.pathname === item.path
      ),
    [location.pathname]
  )

  return (
    <div className="app-shell">
      <RiskAcknowledgementModal />
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

      <div className="app-top-sticky">
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
                <strong className="app-brand-title">CL8Y DEX</strong>
              </span>
            </NavLink>

            <nav className="app-desktop-nav" aria-label="Primary">
              {(fullDesktopHeader ? PRIMARY_NAV_ITEMS : PRIMARY_NAV_ITEMS.slice(0, 1)).map((item) => (
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
                  className={`app-more-trigger${isHeaderMoreRouteActive ? ' app-nav-link-active' : ''}`}
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
                    {headerMoreMenuItems.map((item) => (
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
              {!showMobileLegalStrip ? (
                <ThemeSegmentedControl
                  theme={theme}
                  onSelect={setThemeAndPersist}
                  groupClassName="app-header-theme-group"
                  labelStyle="short"
                />
              ) : null}
              <NetworkBadge />
              <WalletButton />
            </div>
          </div>
        </header>

        <EnvironmentRibbon />
      </div>

      <main className="app-main-shell">
        <div className="app-main">
          <div aria-hidden="true" className="app-hero-glow" />
          <div className="app-main-content">
            {isLcdUnreachable ? <LcdConnectivityBanner onRetry={retryAll} isProbing={isProbePending} /> : null}
            <RouteContentReadyProvider onReadyChange={setRouteContentReady}>
              <Outlet />
            </RouteContentReadyProvider>
          </div>
          {showMobileLegalStrip && routeContentReady ? (
            <div className="app-mobile-legal-strip">
              <LegalFooterNotice />
            </div>
          ) : null}
        </div>
      </main>

      <footer className="app-footer-shell">
        <div className="app-footer">
          <div className="app-footer-copy">
            <p className="app-footer-title">CL8Y DEX · Terra Classic</p>
            {routeContentReady ? <LegalFooterNotice /> : null}
          </div>
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
          className={`app-mobile-more${isMobileMoreSheetRoute ? ' app-mobile-more-active' : ''}`}
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
