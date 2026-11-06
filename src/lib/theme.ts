export interface Palette {
  id: string;
  name: string;
  hex: string;
}

/** Готовые цветовые палитры (акцент) */
export const PALETTES: Palette[] = [
  { id: "violet-nebula", name: "Violet Nebula", hex: "#8b5cf6" },
  { id: "neon-pink", name: "Neon Pink", hex: "#ff2d78" },
  { id: "cyberpunk-yellow", name: "Cyberpunk Yellow", hex: "#ffd60a" },
  { id: "dark-emerald", name: "Dark Emerald", hex: "#17c98a" },
  { id: "electric-blue", name: "Electric Blue", hex: "#4f8bff" },
  { id: "arctic-cyan", name: "Arctic Cyan", hex: "#22d3ee" },
  { id: "inferno-orange", name: "Inferno Orange", hex: "#ff7a2f" },
  { id: "crimson", name: "Crimson", hex: "#ff4d4d" },
];

/** "#rrggbb" -> "r g b" (для CSS rgb(var(--accent-rgb) / a)) */
export function hexToRgbTriplet(hex: string): string {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return "139 92 246";
  const n = parseInt(m[1], 16);
  return `${(n >> 16) & 255} ${(n >> 8) & 255} ${n & 255}`;
}

/** "#rrggbb" -> [r, g, b] */
export function hexToRgb(hex: string): [number, number, number] {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return [139, 92, 246];
  const n = parseInt(m[1], 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

export function rgbToHex(r: number, g: number, b: number): string {
  const c = (v: number) => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, "0");
  return `#${c(r)}${c(g)}${c(b)}`;
}

/** Относительная яркость 0..1 — для выбора контрастного текста на акценте */
export function relLuminance(r: number, g: number, b: number): number {
  const f = (v: number) => {
    const s = v / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
}

/** Тёмный или светлый текст на акцентном фоне */
export function onAccentText(hex: string): string {
  const [r, g, b] = hexToRgb(hex);
  return relLuminance(r, g, b) > 0.4 ? "#0b0c11" : "#ffffff";
}

/** Применить акцентный цвет к документу (CSS-переменные) */
export function applyAccent(hex: string): void {
  const root = document.documentElement;
  root.style.setProperty("--accent-rgb", hexToRgbTriplet(hex));
  root.style.setProperty("--accent-hex", /^#/.test(hex) ? hex : `#${hex}`);
  root.style.setProperty("--accent-ink", onAccentText(hex));
}
