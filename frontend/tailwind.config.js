/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      // The two side hues from the approved layout (issue #7, "Layout decision
      // — dense one-screen board"): attacker amber, defender ice-blue. Carried
      // through headers, tabs and borders so a side is recognisable at a glance.
      colors: {
        attacker: "#E8A33D",
        defender: "#5CA8D8",
      },
    },
  },
  plugins: [],
};
