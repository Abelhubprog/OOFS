// 🎨 OOF Platform Design System - Emotional AI Aesthetics

export const oofDesignSystem = {
  // 🌈 Color Palette with Emotional Psychology
  colors: {
    // Primary brand colors
    primary: {
      50: '#faf5ff',
      100: '#f3e8ff',
      200: '#e9d5ff',
      300: '#d8b4fe',
      400: '#c084fc',
      500: '#a855f7', // Main brand
      600: '#9333ea',
      700: '#7c3aed',
      800: '#6b21a8',
      900: '#581c87',
      950: '#3b0764'
    },

    // Emotional state colors
    emotions: {
      // Regret and pain (red spectrum)
      regret: {
        50: '#fef2f2',
        100: '#fee2e2',
        200: '#fecaca',
        300: '#fca5a5',
        400: '#f87171',
        500: '#ef4444',
        600: '#dc2626',
        700: '#b91c1c',
        800: '#991b1b',
        900: '#7f1d1d'
      },

      // Healing and recovery (green spectrum)
      healing: {
        50: '#f0fdf4',
        100: '#dcfce7',
        200: '#bbf7d0',
        300: '#86efac',
        400: '#4ade80',
        500: '#22c55e',
        600: '#16a34a',
        700: '#15803d',
        800: '#166534',
        900: '#14532d'
      },

      // Viral and excitement (rainbow)
      viral: {
        gradient: 'linear-gradient(135deg, #ff0080 0%, #ff8c00 25%, #40e0d0 50%, #9333ea 75%, #ff0080 100%)',
        glow: 'rgba(255, 0, 128, 0.3)'
      },

      // Therapeutic calm (blue spectrum)
      therapy: {
        50: '#eff6ff',
        100: '#dbeafe',
        200: '#bfdbfe',
        300: '#93c5fd',
        400: '#60a5fa',
        500: '#3b82f6',
        600: '#2563eb',
        700: '#1d4ed8',
        800: '#1e40af',
        900: '#1e3a8a'
      }
    },

    // Surface colors with glass effects
    surfaces: {
      glass: {
        primary: 'rgba(147, 51, 234, 0.1)',
        secondary: 'rgba(124, 58, 237, 0.05)',
        accent: 'rgba(168, 85, 247, 0.15)'
      },

      cards: {
        elevated: 'rgba(147, 51, 234, 0.08)',
        interactive: 'rgba(168, 85, 247, 0.12)',
        hover: 'rgba(147, 51, 234, 0.15)'
      },

      modals: {
        overlay: 'rgba(0, 0, 0, 0.85)',
        content: 'linear-gradient(135deg, #1e1b4b 0%, #312e81 100%)'
      }
    }
  },

  // 📝 Typography with Emotional Weight
  typography: {
    // Display fonts for high emotional impact
    display: {
      fontFamily: '"Space Grotesk", "Inter", system-ui, sans-serif',
      weights: {
        normal: '400',
        medium: '500',
        semibold: '600',
        bold: '700',
        black: '900'
      },
      sizes: {
        xs: '0.75rem',    // 12px
        sm: '0.875rem',   // 14px
        base: '1rem',     // 16px
        lg: '1.125rem',   // 18px
        xl: '1.25rem',    // 20px
        '2xl': '1.5rem',  // 24px
        '3xl': '1.875rem', // 30px
        '4xl': '2.25rem',  // 36px
        '5xl': '3rem',     // 48px
        '6xl': '3.75rem',  // 60px
        '7xl': '4.5rem',   // 72px
      }
    },

    // Narrative fonts for storytelling
    narrative: {
      fontFamily: '"Inter", "Roboto", system-ui, sans-serif',
      lineHeight: '1.7',
      letterSpacing: '0.01em'
    },

    // Interface fonts for UI elements
    interface: {
      fontFamily: '"Inter", system-ui, sans-serif',
      lineHeight: '1.5',
      letterSpacing: '0'
    },

    // Monospace for data and metrics
    data: {
      fontFamily: '"JetBrains Mono", "Fira Code", "Monaco", monospace',
      lineHeight: '1.4',
      letterSpacing: '-0.01em'
    }
  },

  // 🎭 Animation Presets for Emotional States
  animations: {
    // Regret animations (slow, heavy)
    regret: {
      duration: '1200ms',
      easing: 'cubic-bezier(0.68, -0.55, 0.265, 1.55)',
      scale: 'scale(0.95)',
      opacity: 'opacity-60'
    },

    // Healing animations (gentle, soothing)
    healing: {
      duration: '800ms',
      easing: 'cubic-bezier(0.4, 0, 0.2, 1)',
      scale: 'scale(1.02)',
      glow: 'drop-shadow(0 0 20px rgba(34, 197, 94, 0.4))'
    },

    // Viral animations (explosive, energetic)
    viral: {
      duration: '400ms',
      easing: 'cubic-bezier(0.68, -0.55, 0.265, 1.55)',
      scale: 'scale(1.1)',
      rotation: 'rotate(2deg)',
      glow: 'drop-shadow(0 0 30px rgba(255, 0, 128, 0.6))'
    },

    // Therapeutic animations (calm, steady)
    therapeutic: {
      duration: '600ms',
      easing: 'cubic-bezier(0.25, 0.46, 0.45, 0.94)',
      pulse: 'animate-pulse',
      breathe: 'animate-bounce'
    },

    // Loading animations
    loading: {
      spinner: 'animate-spin',
      pulse: 'animate-pulse',
      bounce: 'animate-bounce',
      ping: 'animate-ping'
    }
  },

  // 📏 Spacing Scale
  spacing: {
    px: '1px',
    0: '0',
    0.5: '0.125rem',  // 2px
    1: '0.25rem',     // 4px
    1.5: '0.375rem',  // 6px
    2: '0.5rem',      // 8px
    2.5: '0.625rem',  // 10px
    3: '0.75rem',     // 12px
    3.5: '0.875rem',  // 14px
    4: '1rem',        // 16px
    5: '1.25rem',     // 20px
    6: '1.5rem',      // 24px
    7: '1.75rem',     // 28px
    8: '2rem',        // 32px
    9: '2.25rem',     // 36px
    10: '2.5rem',     // 40px
    11: '2.75rem',    // 44px
    12: '3rem',       // 48px
    14: '3.5rem',     // 56px
    16: '4rem',       // 64px
    20: '5rem',       // 80px
    24: '6rem',       // 96px
    28: '7rem',       // 112px
    32: '8rem',       // 128px
    36: '9rem',       // 144px
    40: '10rem',      // 160px
    44: '11rem',      // 176px
    48: '12rem',      // 192px
    52: '13rem',      // 208px
    56: '14rem',      // 224px
    60: '15rem',      // 240px
    64: '16rem',      // 256px
    72: '18rem',      // 288px
    80: '20rem',      // 320px
    96: '24rem'       // 384px
  },

  // 🔥 Component Variants with Emotional Context
  components: {
    // Button variants
    buttons: {
      // Primary emotional buttons
      primary: {
        base: 'inline-flex items-center justify-center px-6 py-3 rounded-xl font-semibold transition-all duration-300',
        normal: 'bg-gradient-to-r from-purple-600 to-purple-700 hover:from-purple-700 hover:to-purple-800 text-white',
        regret: 'bg-gradient-to-r from-red-600 to-red-700 hover:from-red-700 hover:to-red-800 text-white',
        healing: 'bg-gradient-to-r from-green-600 to-green-700 hover:from-green-700 hover:to-green-800 text-white',
        viral: 'bg-gradient-to-r from-pink-500 via-purple-500 to-cyan-500 hover:scale-105 text-white animate-pulse',
      },

      // Secondary buttons
      secondary: {
        base: 'inline-flex items-center justify-center px-6 py-3 rounded-xl font-medium transition-all duration-300',
        glass: 'bg-white/10 hover:bg-white/20 backdrop-blur-lg border border-white/20 text-white',
        outline: 'border border-purple-500 hover:bg-purple-500/10 text-purple-300 hover:text-white'
      },

      // Therapeutic buttons (gentle, safe)
      therapeutic: {
        base: 'inline-flex items-center justify-center px-6 py-3 rounded-xl font-medium transition-all duration-500',
        calm: 'bg-blue-600/20 hover:bg-blue-600/30 text-blue-200 border border-blue-500/30',
        safe: 'bg-green-600/20 hover:bg-green-600/30 text-green-200 border border-green-500/30'
      }
    },

    // Card variants
    cards: {
      // OOF Moment cards
      moment: {
        base: 'rounded-2xl border backdrop-blur-xl transition-all duration-300',
        legendary: 'bg-gradient-to-br from-yellow-900/20 to-orange-900/20 border-yellow-500/30 shadow-lg shadow-yellow-500/20',
        epic: 'bg-gradient-to-br from-purple-900/20 to-pink-900/20 border-purple-500/30 shadow-lg shadow-purple-500/20',
        rare: 'bg-gradient-to-br from-blue-900/20 to-cyan-900/20 border-blue-500/30 shadow-lg shadow-blue-500/20',
        common: 'bg-gradient-to-br from-gray-900/20 to-gray-800/20 border-gray-500/30 shadow-lg shadow-gray-500/20'
      },

      // Interactive cards
      interactive: {
        base: 'rounded-xl border backdrop-blur-lg transition-all duration-300 cursor-pointer',
        hover: 'hover:scale-105 hover:shadow-xl hover:bg-white/10',
        active: 'active:scale-95'
      },

      // Therapeutic cards (gentle, healing)
      therapeutic: {
        base: 'rounded-xl border backdrop-blur-lg transition-all duration-500',
        healing: 'bg-green-900/10 border-green-500/20 hover:bg-green-900/15',
        supportive: 'bg-blue-900/10 border-blue-500/20 hover:bg-blue-900/15'
      }
    },

    // Modal variants
    modals: {
      base: 'fixed inset-0 z-50 flex items-center justify-center p-4',
      overlay: 'fixed inset-0 bg-black/85 backdrop-blur-md',
      content: 'relative w-full max-w-lg rounded-2xl border border-purple-500/30 bg-gradient-to-br from-purple-900/90 to-purple-800/90 backdrop-blur-xl shadow-2xl',

      // Therapeutic modals (gentler, safer)
      therapeutic: {
        overlay: 'fixed inset-0 bg-black/70 backdrop-blur-sm',
        content: 'relative w-full max-w-lg rounded-2xl border border-blue-500/20 bg-gradient-to-br from-blue-900/80 to-blue-800/80 backdrop-blur-xl'
      }
    },

    // Input variants
    inputs: {
      base: 'w-full px-4 py-3 rounded-xl border backdrop-blur-lg transition-all duration-300',
      default: 'bg-white/5 border-purple-500/30 text-white placeholder:text-purple-300 focus:border-purple-400 focus:bg-white/10',
      error: 'bg-red-900/10 border-red-500/50 text-red-100 placeholder:text-red-300 focus:border-red-400',
      success: 'bg-green-900/10 border-green-500/50 text-green-100 placeholder:text-green-300 focus:border-green-400'
    },

    // Badge variants
    badges: {
      base: 'inline-flex items-center px-3 py-1 rounded-full text-xs font-medium',
      legendary: 'bg-yellow-900/30 text-yellow-300 border border-yellow-500/30',
      epic: 'bg-purple-900/30 text-purple-300 border border-purple-500/30',
      rare: 'bg-blue-900/30 text-blue-300 border border-blue-500/30',
      common: 'bg-gray-900/30 text-gray-300 border border-gray-500/30',

      // Emotional badges
      regret: 'bg-red-900/30 text-red-300 border border-red-500/30',
      healing: 'bg-green-900/30 text-green-300 border border-green-500/30',
      viral: 'bg-gradient-to-r from-pink-600/30 to-purple-600/30 text-pink-200 border border-pink-500/30 animate-pulse'
    }
  },

  // 🌟 Effects and Filters
  effects: {
    // Glass morphism
    glass: {
      light: 'backdrop-blur-sm bg-white/5',
      medium: 'backdrop-blur-md bg-white/10',
      heavy: 'backdrop-blur-lg bg-white/15',
      intense: 'backdrop-blur-xl bg-white/20'
    },

    // Glow effects
    glow: {
      purple: 'drop-shadow(0 0 20px rgba(147, 51, 234, 0.6))',
      viral: 'drop-shadow(0 0 30px rgba(255, 0, 128, 0.8))',
      healing: 'drop-shadow(0 0 25px rgba(34, 197, 94, 0.6))',
      regret: 'drop-shadow(0 0 20px rgba(239, 68, 68, 0.6))'
    },

    // Shadows
    shadows: {
      sm: '0 1px 2px 0 rgba(0, 0, 0, 0.05)',
      default: '0 1px 3px 0 rgba(0, 0, 0, 0.1), 0 1px 2px 0 rgba(0, 0, 0, 0.06)',
      md: '0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)',
      lg: '0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05)',
      xl: '0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04)',
      '2xl': '0 25px 50px -12px rgba(0, 0, 0, 0.25)',
      inner: 'inset 0 2px 4px 0 rgba(0, 0, 0, 0.06)',

      // Emotional shadows
      emotional: {
        regret: '0 10px 30px -5px rgba(239, 68, 68, 0.3)',
        healing: '0 10px 30px -5px rgba(34, 197, 94, 0.3)',
        viral: '0 10px 30px -5px rgba(255, 0, 128, 0.4)',
        therapeutic: '0 8px 25px -3px rgba(59, 130, 246, 0.2)'
      }
    }
  },

  // 📱 Responsive Breakpoints
  breakpoints: {
    xs: '475px',
    sm: '640px',
    md: '768px',
    lg: '1024px',
    xl: '1280px',
    '2xl': '1536px'
  },

  // 🎯 Z-Index Scale
  zIndex: {
    auto: 'auto',
    0: '0',
    10: '10',
    20: '20',
    30: '30',
    40: '40',
    50: '50',
    modal: '1000',
    popover: '1010',
    tooltip: '1020',
    notification: '1030',
    max: '9999'
  }
};

// 🛠️ Utility Functions for Design System
export const getEmotionalColor = (emotion: 'regret' | 'healing' | 'viral' | 'therapeutic', shade: number = 500) => {
  return oofDesignSystem.colors.emotions[emotion][shade] || oofDesignSystem.colors.emotions[emotion][500];
};

export const getComponentVariant = (component: string, variant: string, subVariant?: string) => {
  const comp = oofDesignSystem.components[component as keyof typeof oofDesignSystem.components];
  if (!comp) return '';

  if (subVariant) {
    return comp[variant as keyof typeof comp]?.[subVariant as keyof any] || '';
  }

  return comp[variant as keyof typeof comp] || '';
};

export const getAnimationPreset = (emotion: 'regret' | 'healing' | 'viral' | 'therapeutic') => {
  return oofDesignSystem.animations[emotion];
};

export default oofDesignSystem;
