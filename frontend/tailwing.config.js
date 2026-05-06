/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./index.html", "./src/**/*.{js,html}"],
  theme: {
    extend: {
      colors: {
        'rudi-primary': '#5850ec',
        'rudi-dark': '#0f0f12',
        'rudi-surface': '#1c1c20',
      }
    },
  },
  plugins: [],
}