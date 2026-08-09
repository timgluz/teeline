/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{astro,html,js,jsx,md,mdx,svelte,ts,tsx,vue}'],
  theme: {
    extend: {
      colors: {
        canvas: '#0A0A0B',
        surface: '#141416',
        'surface-2': '#1C1C20',
        border: '#232327',
        'border-hover': '#3A3A40',
        text: '#F5F5F6',
        'text-dim': '#8A8A92',
        accent: '#6366F1',
        'accent-dim': 'rgba(99,102,241,0.12)',
        positive: '#22D3A5',
        negative: '#F87171',
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'ui-monospace', 'monospace'],
      },
      fontSize: {
        xs: '0.8125rem',
        base: '0.9375rem',
        lg: '1.125rem',
        xl: '1.5rem',
        '2xl': '2rem',
        '3xl': '3rem',
      },
      letterSpacing: {
        tight: '-0.02em',
      },
      maxWidth: {
        content: '1200px',
      },
    },
  },
  plugins: [
    require('@tailwindcss/typography'),
  ],
}
