/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      cursor: {
        default: 'var(--cursor-default)',
        pointer: 'var(--cursor-pointer)',
        wait: 'var(--cursor-wait)',
        text: 'var(--cursor-text)',
        'not-allowed': 'var(--cursor-not-allowed)',
        grab: 'var(--cursor-grab)',
        grabbing: 'var(--cursor-grabbing)',
        move: 'var(--cursor-move)',
        progress: 'var(--cursor-wait)',
        auto: 'var(--cursor-default)',
      },
      colors: {
        // Glass system tokens — aliases to CSS variables (docs/design-system.md § Tailwind).
        bg: {
          0: 'var(--bg-0)',
          1: 'var(--bg-1)',
          2: 'var(--bg-2)',
        },
        ink: {
          DEFAULT: 'var(--ink)',
          dim: 'var(--ink-dim)',
          subtle: 'var(--ink-subtle)',
        },
        line: {
          DEFAULT: 'var(--line)',
          strong: 'var(--line-strong)',
        },
        mint: {
          DEFAULT: 'var(--mint)',
          soft: 'var(--mint-soft)',
        },
        // #488: blue = primary CTA (--mint alias); gold = brand / network accents
        blue: {
          DEFAULT: 'var(--blue)',
        },
        gold: {
          DEFAULT: 'var(--gold)',
        },
        accent: 'var(--accent)',
        surface: {
          0: 'var(--surface-0)',
          1: 'var(--surface-1)',
          2: 'var(--surface-2)',
          raised: 'var(--surface-raised)',
        },
        positive: 'var(--color-positive)',
        negative: 'var(--color-negative)',
        warning: 'var(--color-warning)',
      },
    },
  },
  plugins: [],
}
