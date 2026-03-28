// ---------------------------------------------------------------------------
// AgentOS Mobile -- Design system tokens
// Matches the desktop dark theme for visual consistency.
// ---------------------------------------------------------------------------

export const colors = {
  // Backgrounds
  bg: '#0A0E14',
  bgSecondary: '#0D1117',
  bgDeep: '#080B10',
  bgTertiary: '#1A1E26',
  bgElevated: '#1A1E26',

  // Accent / brand — Cyan
  accent: '#00E5E5',
  accentLight: '#4DB8B8',
  accentDark: '#00B8D4',
  accentMuted: 'rgba(0, 229, 229, 0.15)',

  // Semantic
  success: '#2ECC71',
  error: '#E74C3C',
  warning: '#F39C12',
  info: '#378ADD',
  purple: '#5865F2',

  // Text
  text: '#E6EDF3',
  textSecondary: '#C5D0DC',
  textMuted: '#3D4F5F',
  textDim: '#2A3441',
  textInverse: '#0A0E14',

  // Borders
  border: '#1A1E26',
  borderLight: '#2A3441',

  // Chart colors
  chart: ['#00E5E5', '#2ECC71', '#F39C12', '#5865F2', '#E74C3C', '#378ADD'] as readonly string[],

  // Misc
  overlay: 'rgba(0, 0, 0, 0.6)',
  transparent: 'transparent',
} as const;

export const typography = {
  fontFamily: undefined, // use system default on each platform

  sizes: {
    xs: 11,
    sm: 13,
    base: 15,
    md: 17,
    lg: 20,
    xl: 24,
    '2xl': 30,
    '3xl': 36,
  },

  weights: {
    regular: '400' as const,
    medium: '500' as const,
    semibold: '600' as const,
    bold: '700' as const,
  },

  lineHeights: {
    tight: 1.2,
    normal: 1.5,
    relaxed: 1.75,
  },
} as const;

export const spacing = {
  xxs: 2,
  xs: 4,
  sm: 8,
  md: 12,
  base: 16,
  lg: 20,
  xl: 24,
  '2xl': 32,
  '3xl': 40,
  '4xl': 48,
} as const;

export const radii = {
  sm: 6,
  md: 10,
  lg: 14,
  xl: 20,
  full: 9999,
} as const;

export const shadows = {
  sm: {
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 1 },
    shadowOpacity: 0.25,
    shadowRadius: 2,
    elevation: 2,
  },
  md: {
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.3,
    shadowRadius: 4,
    elevation: 4,
  },
  lg: {
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.35,
    shadowRadius: 8,
    elevation: 8,
  },
} as const;
