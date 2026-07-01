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
          deep: '#030303',
          card: '#09090b',
          hover: '#121214',
          border: '#18181b',
          textMuted: '#a1a1aa',
          glow: 'rgba(99, 102, 241, 0.15)',
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
        sans: ['Inter', 'Outfit', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'Fira Code', 'monospace'],
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
