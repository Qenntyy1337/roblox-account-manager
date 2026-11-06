/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: [
          "InterVariable",
          "Inter",
          "Segoe UI Variable Text",
          "Segoe UI",
          "system-ui",
          "sans-serif",
        ],
        mono: ["Cascadia Code", "Consolas", "ui-monospace", "SFMono-Regular", "monospace"],
      },
      colors: {
        bg: "rgb(var(--c-bg) / <alpha-value>)",
        panel: "rgb(var(--c-panel) / <alpha-value>)",
        surface: "rgb(var(--c-surface) / <alpha-value>)",
        raise: "rgb(var(--c-raise) / <alpha-value>)",
        line: "rgb(var(--c-line) / <alpha-value>)",
        ink: "rgb(var(--c-ink) / <alpha-value>)",
        mut: "rgb(var(--c-mut) / <alpha-value>)",
        faint: "rgb(var(--c-faint) / <alpha-value>)",
        accent: "rgb(var(--accent-rgb) / <alpha-value>)",
        ok: "rgb(var(--c-ok) / <alpha-value>)",
        warn: "rgb(var(--c-warn) / <alpha-value>)",
        err: "rgb(var(--c-err) / <alpha-value>)",
        dbg: "rgb(var(--c-dbg) / <alpha-value>)",
        info: "rgb(var(--c-info) / <alpha-value>)",
      },
      borderRadius: {
        xl2: "1.25rem",
      },
      boxShadow: {
        glow: "0 0 24px 0 rgb(var(--accent-rgb) / 0.35)",
        glowSm: "0 0 12px 0 rgb(var(--accent-rgb) / 0.30)",
        card: "0 8px 28px -10px rgb(0 0 0 / 0.55)",
        modal: "0 24px 80px -12px rgb(0 0 0 / 0.75)",
      },
      keyframes: {
        fadeUp: {
          "0%": { opacity: "0", transform: "translateY(10px)" },
          "100%": { opacity: "1", transform: "translateY(0)" },
        },
        scaleIn: {
          "0%": { opacity: "0", transform: "scale(0.96) translateY(6px)" },
          "100%": { opacity: "1", transform: "scale(1) translateY(0)" },
        },
        fadeIn: {
          "0%": { opacity: "0" },
          "100%": { opacity: "1" },
        },
        pulseDot: {
          "0%, 100%": { opacity: "1", transform: "scale(1)" },
          "50%": { opacity: "0.45", transform: "scale(0.82)" },
        },
        slideIn: {
          "0%": { opacity: "0", transform: "translateX(16px)" },
          "100%": { opacity: "1", transform: "translateX(0)" },
        },
      },
      animation: {
        fadeUp: "fadeUp 0.28s cubic-bezier(0.22, 0.9, 0.3, 1) both",
        scaleIn: "scaleIn 0.22s cubic-bezier(0.22, 0.9, 0.3, 1.05) both",
        fadeIn: "fadeIn 0.18s ease both",
        pulseDot: "pulseDot 1.6s ease-in-out infinite",
        slideIn: "slideIn 0.25s cubic-bezier(0.22, 0.9, 0.3, 1) both",
      },
    },
  },
  plugins: [],
};
