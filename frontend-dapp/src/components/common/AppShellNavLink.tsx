import { Link, useLocation, useMatch, useNavigate } from 'react-router-dom'
import type { NavItem } from '@/components/common/navItems'
import { sounds } from '@/lib/sounds'

function isModifiedClick(event: React.MouseEvent<HTMLAnchorElement>): boolean {
  return (
    event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey || event.defaultPrevented
  )
}

export type AppShellNavLinkProps = {
  item: NavItem
  className: (args: { isActive: boolean }) => string
  /** Optional hook after press sound (e.g. close More menu). */
  onAfterPress?: () => void
  role?: string
  labelClassName?: string
}

/**
 * Shell navigation link with explicit `navigate()` on plain left-click.
 * Wallet extensions and some browsers can swallow NavLink's default handler while
 * still painting :active styles — URL and Outlet then stay stale (GitLab #182).
 */
export function AppShellNavLink({
  item,
  className,
  onAfterPress,
  role,
  labelClassName = 'app-nav-link-label',
}: AppShellNavLinkProps) {
  const navigate = useNavigate()
  const location = useLocation()
  const isActive = useMatch({ path: item.path, end: item.end !== false }) != null

  return (
    <Link
      to={item.path}
      role={role}
      className={className({ isActive })}
      onClick={(event) => {
        sounds.playButtonPress()
        onAfterPress?.()
        if (isModifiedClick(event)) return
        event.preventDefault()
        if (location.pathname !== item.path) {
          navigate(item.path)
        }
      }}
    >
      <span className={labelClassName}>{item.label}</span>
    </Link>
  )
}
