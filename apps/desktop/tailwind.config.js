/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        brand: {
          deep: 'var(--color-brand-deep)',
          card: 'var(--color-brand-card)',
          hover: 'var(--color-brand-hover)',
          border: 'var(--color-brand-border)',
          textMuted: 'var(--color-brand-text-muted)',
          glow: 'var(--color-brand-glow)',
        },
        accent: {
          primary: '#6366f1',    // Indigo for core actions
          secondary: '#a855f7',  // Purple for special items
          success: '#10b981',    // Emerald for success/active states
          warning: '#f59e0b',    // Amber for warnings
          danger: '#f43f5e',     // Rose for errors/destructive actions
        }
      },
      fontFamily: {
        sans: ['Inter', 'Outfit', '"Segoe UI Variable Text"', '"Segoe UI"', 'system-ui', '-apple-system', 'sans-serif'],
        mono: ['"JetBrains Mono"', '"Fira Code"', 'Consolas', '"Courier New"', 'monospace'],
      },
      animation: {
        'pulse-slow': 'pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'glow-pulse': 'glow 2s ease-in-out infinite alternate',
      },
      keyframes: {
        glow: {
          '0%': { boxShadow: '0 0 5px rgba(99, 102, 241, 0.2), 0 0 10px rgba(99, 102, 241, 0.1)' },
          '100%': { boxShadow: '0 0 15px rgba(99, 102, 241, 0.4), 0 0 25px rgba(99, 102, 241, 0.2)' },
        }
      }
    },
  },
  plugins: [],
}
