import { CL8Y_PRODUCT_LINKS, canonicalCl8yProductHref } from '@/utils/cl8yProductLinks'

/**
 * Compact official CL8Y product row for the shell footer (GitLab #663).
 * Text-only (no logo) so this cannot become LCP. Renders immediately — legal
 * NFA stays deferred until route-ready (#179).
 */
export default function Cl8yProductLinks() {
  return (
    <nav className="app-footer-product-links" aria-label="CL8Y products">
      {CL8Y_PRODUCT_LINKS.map((link) => {
        const href = canonicalCl8yProductHref(link.href)
        if (!href) return null
        return (
          <a
            key={link.id}
            className="app-footer-product-link"
            href={href}
            target="_blank"
            rel="noopener noreferrer"
            data-testid={link.testId}
          >
            {link.label}
          </a>
        )
      })}
    </nav>
  )
}
