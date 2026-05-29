/**
 * Vitest setup for real `lightweight-charts` suites (vitest.config.charts.ts).
 * Does not load lightweightChartsJsdomMock — patches Canvas 2D via the `canvas` package.
 */
import '@testing-library/jest-dom'
import { createCanvas } from 'canvas'
import { afterEach } from 'vitest'
import { cleanup } from '@testing-library/react'

afterEach(() => {
  cleanup()
})

const nativeGetContext = HTMLCanvasElement.prototype.getContext

HTMLCanvasElement.prototype.getContext = function (
  contextId: string,
  contextAttributes?: CanvasRenderingContext2DSettings
): RenderingContext | null {
  if (contextId === '2d') {
    const w = this.width > 0 ? this.width : 640
    const h = this.height > 0 ? this.height : 400
    const nodeCanvas = createCanvas(w, h)
    return nodeCanvas.getContext('2d', contextAttributes) as RenderingContext | null
  }
  return nativeGetContext.call(this, contextId, contextAttributes)
}

/** jsdom often reports 0×0; derive size from inline styles so lightweight-charts can init. */
function defineClientSizeProp(prop: 'clientWidth' | 'clientHeight', styleProp: 'width' | 'height', fallback: number) {
  const existing = Object.getOwnPropertyDescriptor(HTMLElement.prototype, prop)
  if (existing && !existing.configurable) return

  Object.defineProperty(HTMLElement.prototype, prop, {
    configurable: true,
    get(this: HTMLElement) {
      const own = this.style?.[styleProp]
      if (own?.endsWith('px')) return parseInt(own, 10)
      const parent = this.parentElement
      const parentSize = parent?.style?.[styleProp]
      if (parentSize?.endsWith('px')) return parseInt(parentSize, 10)
      return fallback
    },
  })
}

function hexToRgbString(hex: string): string | null {
  const normalized = hex.trim().replace(/^#/, '')
  if (!/^[0-9a-f]{6}$/i.test(normalized)) return null
  const r = parseInt(normalized.slice(0, 2), 16)
  const g = parseInt(normalized.slice(2, 4), 16)
  const b = parseInt(normalized.slice(4, 6), 16)
  return `rgb(${r}, ${g}, ${b})`
}

if (typeof window !== 'undefined') {
  defineClientSizeProp('clientWidth', 'width', 640)
  defineClientSizeProp('clientHeight', 'height', 400)

  const nativeGetComputedStyle = window.getComputedStyle.bind(window)
  window.getComputedStyle = (element: Element, pseudoElt?: string | null) => {
    const declaration = nativeGetComputedStyle(element, pseudoElt)
    const inlineColor = (element as HTMLElement).style?.color
    const rgb = inlineColor ? hexToRgbString(inlineColor) : null
    if (rgb) {
      return { ...declaration, color: rgb } as CSSStyleDeclaration
    }
    return declaration
  }

  if (!window.ResizeObserver) {
    window.ResizeObserver = class ResizeObserver {
      observe() {}
      unobserve() {}
      disconnect() {}
    }
  }

  if (!window.IntersectionObserver) {
    window.IntersectionObserver = class IntersectionObserver {
      root = null
      rootMargin = ''
      thresholds: number[] = []
      observe() {}
      unobserve() {}
      disconnect() {}
      takeRecords() {
        return []
      }
    } as unknown as typeof IntersectionObserver
  }

  if (!window.matchMedia) {
    window.matchMedia = (query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    })
  }
}
