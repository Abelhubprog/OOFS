import React, { createContext, useContext, useEffect, useState, ReactNode } from 'react';
import { oofDesignSystem, getEmotionalColor, getComponentVariant, getAnimationPreset } from '@/lib/design-system';

// 🎨 Theme Context Types
interface ThemeContextType {
  // Current theme state
  theme: 'dark' | 'light';
  emotionalMode: 'normal' | 'therapeutic' | 'viral' | 'regret';

  // Theme control methods
  setTheme: (theme: 'dark' | 'light') => void;
  setEmotionalMode: (mode: 'normal' | 'therapeutic' | 'viral' | 'regret') => void;

  // Design system utilities
  colors: typeof oofDesignSystem.colors;
  getEmotionalColor: typeof getEmotionalColor;
  getComponentVariant: typeof getComponentVariant;
  getAnimationPreset: typeof getAnimationPreset;

  // Accessibility settings
  reducedMotion: boolean;
  highContrast: boolean;
  setReducedMotion: (enabled: boolean) => void;
  setHighContrast: (enabled: boolean) => void;

  // Therapeutic features
  therapeuticMode: boolean;
  setTherapeuticMode: (enabled: boolean) => void;
}

// Create Theme Context
const ThemeContext = createContext<ThemeContextType | null>(null);

// 🌈 Theme Provider Component
export function OOFThemeProvider({ children }: { children: ReactNode }) {
  // Theme state
  const [theme, setTheme] = useState<'dark' | 'light'>('dark');
  const [emotionalMode, setEmotionalMode] = useState<'normal' | 'therapeutic' | 'viral' | 'regret'>('normal');

  // Accessibility state
  const [reducedMotion, setReducedMotion] = useState(false);
  const [highContrast, setHighContrast] = useState(false);
  const [therapeuticMode, setTherapeuticMode] = useState(false);

  // Initialize theme from localStorage and system preferences
  useEffect(() => {
    // Load saved theme preferences
    const savedTheme = localStorage.getItem('oof-theme') as 'dark' | 'light' | null;
    const savedEmotionalMode = localStorage.getItem('oof-emotional-mode') as 'normal' | 'therapeutic' | 'viral' | 'regret' | null;
    const savedTherapeuticMode = localStorage.getItem('oof-therapeutic-mode') === 'true';

    // Apply saved preferences or defaults
    if (savedTheme) {
      setTheme(savedTheme);
    } else {
      // Default to dark theme for OOF Platform
      setTheme('dark');
    }

    if (savedEmotionalMode) {
      setEmotionalMode(savedEmotionalMode);
    }

    setTherapeuticMode(savedTherapeuticMode);

    // Check for system accessibility preferences
    const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    const prefersHighContrast = window.matchMedia('(prefers-contrast: high)').matches;

    setReducedMotion(prefersReducedMotion);
    setHighContrast(prefersHighContrast);

    // Listen for system preference changes
    const motionMediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
    const contrastMediaQuery = window.matchMedia('(prefers-contrast: high)');

    const handleMotionChange = (e: MediaQueryListEvent) => setReducedMotion(e.matches);
    const handleContrastChange = (e: MediaQueryListEvent) => setHighContrast(e.matches);

    motionMediaQuery.addEventListener('change', handleMotionChange);
    contrastMediaQuery.addEventListener('change', handleContrastChange);

    return () => {
      motionMediaQuery.removeEventListener('change', handleMotionChange);
      contrastMediaQuery.removeEventListener('change', handleContrastChange);
    };
  }, []);

  // Apply theme to document
  useEffect(() => {
    const root = document.documentElement;

    // Apply theme class
    root.classList.remove('light', 'dark');
    root.classList.add(theme);

    // Apply emotional mode class
    root.classList.remove('normal-mode', 'therapeutic-mode', 'viral-mode', 'regret-mode');
    root.classList.add(`${emotionalMode}-mode`);

    // Apply accessibility classes
    if (reducedMotion) {
      root.classList.add('reduced-motion');
    } else {
      root.classList.remove('reduced-motion');
    }

    if (highContrast) {
      root.classList.add('high-contrast');
    } else {
      root.classList.remove('high-contrast');
    }

    if (therapeuticMode) {
      root.classList.add('therapeutic-mode');
    } else {
      root.classList.remove('therapeutic-mode');
    }

    // Apply CSS custom properties for design system
    const colors = oofDesignSystem.colors;

    // Primary colors
    root.style.setProperty('--oof-primary-50', colors.primary[50]);
    root.style.setProperty('--oof-primary-100', colors.primary[100]);
    root.style.setProperty('--oof-primary-200', colors.primary[200]);
    root.style.setProperty('--oof-primary-300', colors.primary[300]);
    root.style.setProperty('--oof-primary-400', colors.primary[400]);
    root.style.setProperty('--oof-primary-500', colors.primary[500]);
    root.style.setProperty('--oof-primary-600', colors.primary[600]);
    root.style.setProperty('--oof-primary-700', colors.primary[700]);
    root.style.setProperty('--oof-primary-800', colors.primary[800]);
    root.style.setProperty('--oof-primary-900', colors.primary[900]);

    // Emotional colors based on current mode
    const emotionalColors = colors.emotions;

    switch (emotionalMode) {
      case 'regret':
        root.style.setProperty('--oof-emotional-primary', emotionalColors.regret[500]);
        root.style.setProperty('--oof-emotional-secondary', emotionalColors.regret[600]);
        root.style.setProperty('--oof-emotional-accent', emotionalColors.regret[400]);
        break;
      case 'healing':
        root.style.setProperty('--oof-emotional-primary', emotionalColors.healing[500]);
        root.style.setProperty('--oof-emotional-secondary', emotionalColors.healing[600]);
        root.style.setProperty('--oof-emotional-accent', emotionalColors.healing[400]);
        break;
      case 'viral':
        root.style.setProperty('--oof-emotional-primary', '#ff0080');
        root.style.setProperty('--oof-emotional-secondary', '#9333ea');
        root.style.setProperty('--oof-emotional-accent', '#40e0d0');
        break;
      case 'therapeutic':
        root.style.setProperty('--oof-emotional-primary', emotionalColors.therapy[500]);
        root.style.setProperty('--oof-emotional-secondary', emotionalColors.therapy[600]);
        root.style.setProperty('--oof-emotional-accent', emotionalColors.therapy[400]);
        break;
      default:
        root.style.setProperty('--oof-emotional-primary', colors.primary[500]);
        root.style.setProperty('--oof-emotional-secondary', colors.primary[600]);
        root.style.setProperty('--oof-emotional-accent', colors.primary[400]);
    }

    // Glass and surface colors
    root.style.setProperty('--oof-glass-primary', colors.surfaces.glass.primary);
    root.style.setProperty('--oof-glass-secondary', colors.surfaces.glass.secondary);
    root.style.setProperty('--oof-glass-accent', colors.surfaces.glass.accent);

    // Animation durations based on emotional mode and accessibility
    const animations = oofDesignSystem.animations;
    const currentAnimation = animations[emotionalMode] || animations.therapeutic;

    if (reducedMotion) {
      root.style.setProperty('--oof-animation-duration', '0ms');
      root.style.setProperty('--oof-animation-easing', 'linear');
    } else {
      root.style.setProperty('--oof-animation-duration', currentAnimation.duration);
      root.style.setProperty('--oof-animation-easing', currentAnimation.easing);
    }

  }, [theme, emotionalMode, reducedMotion, highContrast, therapeuticMode]);

  // Theme control handlers
  const handleSetTheme = (newTheme: 'dark' | 'light') => {
    setTheme(newTheme);
    localStorage.setItem('oof-theme', newTheme);
  };

  const handleSetEmotionalMode = (mode: 'normal' | 'therapeutic' | 'viral' | 'regret') => {
    setEmotionalMode(mode);
    localStorage.setItem('oof-emotional-mode', mode);

    // Trigger haptic feedback if available
    if ('vibrate' in navigator) {
      navigator.vibrate(50);
    }

    // Track emotional mode change
    if (window.gtag) {
      window.gtag('event', 'emotional_mode_change', {
        previous_mode: emotionalMode,
        new_mode: mode
      });
    }
  };

  const handleSetReducedMotion = (enabled: boolean) => {
    setReducedMotion(enabled);
    localStorage.setItem('oof-reduced-motion', enabled.toString());
  };

  const handleSetHighContrast = (enabled: boolean) => {
    setHighContrast(enabled);
    localStorage.setItem('oof-high-contrast', enabled.toString());
  };

  const handleSetTherapeuticMode = (enabled: boolean) => {
    setTherapeuticMode(enabled);
    localStorage.setItem('oof-therapeutic-mode', enabled.toString());

    // When therapeutic mode is enabled, also enable reduced motion
    if (enabled) {
      setReducedMotion(true);
    }
  };

  // Context value
  const contextValue: ThemeContextType = {
    theme,
    emotionalMode,
    setTheme: handleSetTheme,
    setEmotionalMode: handleSetEmotionalMode,
    colors: oofDesignSystem.colors,
    getEmotionalColor,
    getComponentVariant,
    getAnimationPreset,
    reducedMotion,
    highContrast,
    setReducedMotion: handleSetReducedMotion,
    setHighContrast: handleSetHighContrast,
    therapeuticMode,
    setTherapeuticMode: handleSetTherapeuticMode
  };

  return (
    <ThemeContext.Provider value={contextValue}>
      {children}
    </ThemeContext.Provider>
  );
}

// 🎨 Hook to use theme context
export function useOOFTheme(): ThemeContextType {
  const context = useContext(ThemeContext);

  if (!context) {
    throw new Error('useOOFTheme must be used within OOFThemeProvider');
  }

  return context;
}

// 🎭 Emotional Mode Component
export function EmotionalModeToggle() {
  const { emotionalMode, setEmotionalMode } = useOOFTheme();

  const modes = [
    {
      id: 'normal',
      label: 'Normal',
      icon: '😊',
      description: 'Standard OOF experience'
    },
    {
      id: 'therapeutic',
      label: 'Therapeutic',
      icon: '🧘',
      description: 'Gentle, healing-focused interface'
    },
    {
      id: 'viral',
      label: 'Viral',
      icon: '🚀',
      description: 'High-energy, explosive mode'
    },
    {
      id: 'regret',
      label: 'Regret',
      icon: '💔',
      description: 'Process those painful losses'
    }
  ] as const;

  return (
    <div className="flex gap-2 p-1 bg-black/20 rounded-xl">
      {modes.map((mode) => (
        <button
          key={mode.id}
          onClick={() => setEmotionalMode(mode.id)}
          className={`
            flex items-center gap-2 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-300
            ${emotionalMode === mode.id
              ? 'bg-white/20 text-white shadow-lg'
              : 'text-white/70 hover:text-white hover:bg-white/10'
            }
          `}
          title={mode.description}
        >
          <span className="text-lg">{mode.icon}</span>
          <span className="hidden sm:inline">{mode.label}</span>
        </button>
      ))}
    </div>
  );
}

// 🔧 Accessibility Controls Component
export function AccessibilityControls() {
  const {
    reducedMotion,
    setReducedMotion,
    highContrast,
    setHighContrast,
    therapeuticMode,
    setTherapeuticMode
  } = useOOFTheme();

  return (
    <div className="space-y-4 p-4 bg-black/10 rounded-xl">
      <h3 className="text-lg font-semibold text-white">Accessibility Settings</h3>

      <div className="space-y-3">
        <label className="flex items-center justify-between">
          <span className="text-white/80">Reduce Motion</span>
          <button
            onClick={() => setReducedMotion(!reducedMotion)}
            className={`
              w-12 h-6 rounded-full transition-colors duration-300
              ${reducedMotion ? 'bg-green-500' : 'bg-gray-600'}
            `}
          >
            <div className={`
              w-5 h-5 bg-white rounded-full shadow transition-transform duration-300
              ${reducedMotion ? 'translate-x-6' : 'translate-x-0.5'}
            `} />
          </button>
        </label>

        <label className="flex items-center justify-between">
          <span className="text-white/80">High Contrast</span>
          <button
            onClick={() => setHighContrast(!highContrast)}
            className={`
              w-12 h-6 rounded-full transition-colors duration-300
              ${highContrast ? 'bg-blue-500' : 'bg-gray-600'}
            `}
          >
            <div className={`
              w-5 h-5 bg-white rounded-full shadow transition-transform duration-300
              ${highContrast ? 'translate-x-6' : 'translate-x-0.5'}
            `} />
          </button>
        </label>

        <label className="flex items-center justify-between">
          <span className="text-white/80">Therapeutic Mode</span>
          <button
            onClick={() => setTherapeuticMode(!therapeuticMode)}
            className={`
              w-12 h-6 rounded-full transition-colors duration-300
              ${therapeuticMode ? 'bg-green-500' : 'bg-gray-600'}
            `}
          >
            <div className={`
              w-5 h-5 bg-white rounded-full shadow transition-transform duration-300
              ${therapeuticMode ? 'translate-x-6' : 'translate-x-0.5'}
            `} />
          </button>
        </label>
      </div>

      <p className="text-xs text-white/60">
        Therapeutic mode enables gentle animations and healing-focused colors to support emotional well-being.
      </p>
    </div>
  );
}

export default OOFThemeProvider;
