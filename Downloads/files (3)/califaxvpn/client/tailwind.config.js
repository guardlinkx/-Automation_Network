/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./src/renderer/**/*.{html,tsx,ts}'],
  theme: {
    extend: {
      colors: {
        califax: {
          bg: '#0a0a0f',
          surface: '#12121a',
          border: '#1e1e2e',
          accent: '#00E5A0',
          'accent-hover': '#00cc8e',
          text: '#e4e4e7',
          muted: '#71717a',
        },
      },
    },
  },
  plugins: [],
};
