import { Outlet, NavLink, useLocation } from 'react-router-dom'
import { AppShellNavLink } from '@/components/common/AppShellNavLink'
import { useEffect, useMemo, useRef, useState } from 'react'
import WalletButton from '@/components/wallet/WalletButton'
import WalletConnectPairingModal from '@/components/wallet/WalletConnectPairingModal'
import NetworkBadge from '@/components/wallet/NetworkBadge'
import EnvironmentRibbon from '@/components/legal/EnvironmentRibbon'
import LegalFooterNotice from '@/components/legal/LegalFooterNotice'
import RiskAcknowledgementModal from '@/components/legal/RiskAcknowledgementModal'
import ConnectedTermsGate from '@/components/legal/ConnectedTermsGate'
import { ThemeSegmentedControl, type ThemeMode } from '@/components/common/ThemeSegmentedControl'
import { SoundEffectsToggle } from '@/components/common/SoundEffectsToggle'
import {
  getHeaderMoreMenuItems,
  getMobileMoreMenuItems,
  HEADER_FULL_NAV_MIN_WIDTH_PX,
  MOBILE_BOTTOM_NAV_ITEMS,
  PRIMARY_NAV_ITEMS,
} from '@/components/common/navItems'
import {
  FAUCET_CONTRACT_ADDRESS,
  isCommunityTaxEnabled,
  isNativeWrapEnabled,
  isUst1WindowEnabled,
} from '@/utils/constants'
import { readSoundsEnabled, writeSoundsEnabled } from '@/utils/soundPreferences'
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
  const [soundsEnabled, setSoundsEnabled] = useState(readSoundsEnabled)
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

  /** Persist first; mute click stays silent; unmute plays one confirmation press (#487). */
  const setSoundsEnabledAndPersist = (enabled: boolean) => {
    writeSoundsEnabled(enabled)
    setSoundsEnabled(enabled)
    if (enabled) sounds.playButtonPress()
  }

  useEffect(() => {
    setIsMoreMenuOpen(false)
    setIsMobileMoreOpen(false)
  }, [location.pathname])

  const headerMoreMenuItems = useMemo(
    () =>
      getHeaderMoreMenuItems(fullDesktopHeader, {
        includeMint: !!FAUCET_CONTRACT_ADDRESS,
        includeUst1: isUst1WindowEnabled(),
        includeWrap: isNativeWrapEnabled(),
        includeCreateToken: isCommunityTaxEnabled(),
        includeMigrateToken: isCommunityTaxEnabled(),
      }),
    [fullDesktopHeader]
  )

  const isHeaderMoreRouteActive = useMemo(
    () =>
      headerMoreMenuItems.some((item) =>
        item.end === false ? location.pathname.startsWith(item.path) : location.pathname === item.path
      ),
    [location.pathname, headerMoreMenuItems]
  )

  const mobileMoreMenuItems = useMemo(
    () =>
      getMobileMoreMenuItems({
        includeMint: !!FAUCET_CONTRACT_ADDRESS,
        includeUst1: isUst1WindowEnabled(),
        includeWrap: isNativeWrapEnabled(),
        includeCreateToken: isCommunityTaxEnabled(),
        includeMigrateToken: isCommunityTaxEnabled(),
      }),
    []
  )

  const isMobileMoreSheetRoute = useMemo(
    () =>
      mobileMoreMenuItems.some((item) =>
        item.end === false ? location.pathname.startsWith(item.path) : location.pathname === item.path
      ),
    [location.pathname, mobileMoreMenuItems]
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
                <img src="/logo.png" alt="CL8Y DEX" className="app-brand-logo" />
              </span>
              <span className="app-brand-copy">
                <strong className="app-brand-title">CL8Y DEX</strong>
              </span>
            </NavLink>

            <nav className="app-desktop-nav" aria-label="Primary">
              {(fullDesktopHeader ? PRIMARY_NAV_ITEMS : PRIMARY_NAV_ITEMS.slice(0, 1)).map((item) => (
                <AppShellNavLink
                  key={item.path}
                  item={item}
                  className={({ isActive }) => `app-nav-link${isActive ? ' app-nav-link-active' : ''}`}
                />
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
                      <AppShellNavLink
                        key={item.path}
                        item={item}
                        role="menuitem"
                        onAfterPress={() => setIsMoreMenuOpen(false)}
                        className={({ isActive }) => `app-menu-link${isActive ? ' app-nav-link-active' : ''}`}
                        labelClassName=""
                      />
                    ))}
                  </div>
                )}
              </div>
            </nav>

            <div className="app-header-controls">
              {!showMobileLegalStrip ? (
                <div className="app-header-pref-group" role="group" aria-label="Display and sound">
                  <ThemeSegmentedControl
                    theme={theme}
                    onSelect={setThemeAndPersist}
                    groupClassName="app-header-theme-group"
                    labelStyle="short"
                  />
                  <SoundEffectsToggle
                    enabled={soundsEnabled}
                    onToggle={setSoundsEnabledAndPersist}
                    labelStyle="short"
                    className="app-header-sound-toggle"
                  />
                </div>
              ) : null}
              {/*
                EnvironmentRibbon lives in the footer on all breakpoints (shell density).
                Omit desktop NetworkBadge so theme + wallet keep ≥ ~8px gap from More (#483).
                Mobile keeps the badge next to the wallet chip.
              */}
              {showMobileLegalStrip ? <NetworkBadge /> : null}
              <WalletButton />
            </div>
          </div>
        </header>
      </div>

      <main className="app-main-shell">
        <div className="app-main">
          <div aria-hidden="true" className="app-hero-glow" />
          <div className="app-main-content">
            {isLcdUnreachable ? <LcdConnectivityBanner onRetry={retryAll} isProbing={isProbePending} /> : null}
            <RouteContentReadyProvider onReadyChange={setRouteContentReady}>
              {/* Wallet-bound Legal clickwrap after connect (GitLab #517); header/footer stay usable to disconnect. */}
              <ConnectedTermsGate>
                {/* Remount matched route on tab change so lazy Outlet content cannot stick on the prior page (GitLab #138, #182). */}
                <Outlet key={location.pathname} />
              </ConnectedTermsGate>
            </RouteContentReadyProvider>
          </div>
        </div>
      </main>

      <footer className="app-footer-shell">
        <div className="app-footer">
          <EnvironmentRibbon />
          <div className="app-footer-copy">
            <p className="app-footer-title">CL8Y DEX · Terra Classic</p>
            {routeContentReady ? <LegalFooterNotice /> : null}
          </div>
        </div>
      </footer>

      <nav ref={mobileNavRef} className="app-mobile-nav-shell" aria-label="Primary">
        {MOBILE_BOTTOM_NAV_ITEMS.map((item) => (
          <AppShellNavLink
            key={item.path}
            item={item}
            className={({ isActive }) => `app-mobile-link${isActive ? ' app-mobile-link-active' : ''}`}
            labelClassName=""
          />
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
          {mobileMoreMenuItems.map((item) => (
            <AppShellNavLink
              key={item.path}
              item={item}
              onAfterPress={() => setIsMobileMoreOpen(false)}
              className={({ isActive }) => `app-menu-link${isActive ? ' app-nav-link-active' : ''}`}
              labelClassName=""
            />
          ))}
          <div className="app-mobile-pref-group" role="group" aria-label="Display and sound">
            <ThemeSegmentedControl
              theme={theme}
              onSelect={setThemeAndPersist}
              groupClassName="app-mobile-theme-group"
              labelStyle="long"
            />
            <SoundEffectsToggle
              enabled={soundsEnabled}
              onToggle={setSoundsEnabledAndPersist}
              labelStyle="long"
              className="app-mobile-sound-toggle"
            />
          </div>
        </section>
      )}
      <WalletConnectPairingModal />
    </div>
  )
}
