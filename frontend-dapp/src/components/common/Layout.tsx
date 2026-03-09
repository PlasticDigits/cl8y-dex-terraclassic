import { Outlet, Link, useLocation } from 'react-router-dom'
import WalletButton from '@/components/wallet/WalletButton'

const NAV_ITEMS = [
  { path: '/', label: 'Swap' },
  { path: '/pool', label: 'Pool' },
  { path: '/tiers', label: 'Fee Tiers' },
  { path: '/create', label: 'Create Pair' },
]

export default function Layout() {
  const location = useLocation()

  return (
    <div className="min-h-screen flex flex-col">
      <header className="border-b border-dex-border px-6 py-4">
        <div className="max-w-7xl mx-auto flex items-center justify-between">
          <div className="flex items-center gap-8">
            <Link to="/" className="text-xl font-bold text-dex-accent">CL8Y DEX</Link>
            <nav className="flex gap-4">
              {NAV_ITEMS.map(({ path, label }) => (
                <Link
                  key={path}
                  to={path}
                  className={`text-sm font-medium transition-colors ${
                    location.pathname === path
                      ? 'text-white'
                      : 'text-gray-400 hover:text-gray-200'
                  }`}
                >
                  {label}
                </Link>
              ))}
            </nav>
          </div>
          <WalletButton />
        </div>
      </header>
      <main className="flex-1 max-w-7xl mx-auto w-full px-6 py-8">
        <Outlet />
      </main>
      <footer className="border-t border-dex-border px-6 py-4 text-center text-xs text-gray-500">
        CL8Y DEX on Terra Classic
      </footer>
    </div>
  )
}
