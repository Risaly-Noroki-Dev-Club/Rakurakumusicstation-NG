// Material You dynamic color (materialkolor approach, official JS port).
//
// A seed color is converted to HCT and run through the designer-tuned tonal
// palettes (themeFromSourceColor = SchemeTonalSpot); the resulting primary
// colors for the light and dark schemes become the app's accent pair. The
// pair is cached per seed (computed results are deterministic).

import { themeFromSourceColor } from '@material/material-color-utilities'

export interface AccentTheme {
  /** User-picked seed color (#rrggbb). */
  seed: string
  /** Material light scheme primary (#rrggbb). */
  light: string
  /** Material dark scheme primary (#rrggbb). */
  dark: string
}

export interface AccentSeed {
  seed: string
  name: string
}

const cache = new Map<string, AccentTheme>()

function hexToArgb(hex: string): number {
  const h = hex.replace('#', '').trim()
  const rgb = parseInt(h, 16)
  if (Number.isNaN(rgb)) return 0xff764ba2
  return (0xff << 24) | rgb
}

function argbToHex(argb: number): string {
  return `#${(argb & 0xffffff).toString(16).padStart(6, '0')}`
}

/** Seed → { seed, light, dark } via the Material 3 tonal palettes. */
export function dynamicAccent(seed: string): AccentTheme {
  const normalized = seed.toLowerCase().startsWith('#') ? seed.toLowerCase() : `#${seed.toLowerCase()}`
  const cached = cache.get(normalized)
  if (cached) return cached
  const theme = themeFromSourceColor(hexToArgb(normalized))
  const pair: AccentTheme = {
    seed: normalized,
    light: argbToHex(theme.schemes.light.primary),
    dark: argbToHex(theme.schemes.dark.primary),
  }
  cache.set(normalized, pair)
  return pair
}

export const ACCENT_SEEDS: AccentSeed[] = [
  { seed: '#764ba2', name: '电台紫' },
  { seed: '#5b6ee1', name: '靛蓝' },
  { seed: '#0891b2', name: '海盐青' },
  { seed: '#059669', name: '翡翠' },
  { seed: '#d97706', name: '琥珀' },
  { seed: '#ef4444', name: '珊瑚' },
  { seed: '#db2777', name: '玫红' },
  { seed: '#e2725b', name: '暖橙' },
  { seed: '#475569', name: '石墨' },
]

export const DEFAULT_ACCENT: AccentTheme = dynamicAccent(ACCENT_SEEDS[0].seed)
