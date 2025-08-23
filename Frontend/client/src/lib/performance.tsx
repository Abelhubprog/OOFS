// 🚀 OOF Platform Performance Optimization System

import React, { Suspense, lazy, ComponentType, ReactNode, useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';

// 🎭 Performance-Optimized Loading Components
export function OOFLoadingSpinner({
  message = "Loading your OOF experience...",
  emotionalMode = "normal"
}: {
  message?: string;
  emotionalMode?: 'normal' | 'therapeutic' | 'viral' | 'regret';
}) {
  const getLoadingStyle = () => {
    switch (emotionalMode) {
      case 'therapeutic':
        return {
          spinner: 'border-blue-400',
          bg: 'from-blue-900/20 to-blue-800/20',
          text: 'text-blue-200'
        };
      case 'viral':
        return {
          spinner: 'border-pink-400',
          bg: 'from-pink-900/20 to-purple-900/20',
          text: 'text-pink-200'
        };
      case 'regret':
        return {
          spinner: 'border-red-400',
          bg: 'from-red-900/20 to-red-800/20',
          text: 'text-red-200'
        };
      default:
        return {
          spinner: 'border-purple-400',
          bg: 'from-purple-900/20 to-purple-800/20',
          text: 'text-purple-200'
        };
    }
  };

  const style = getLoadingStyle();

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.9 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: 0.9 }}
      className={`flex flex-col items-center justify-center min-h-[200px] bg-gradient-to-br ${style.bg} rounded-xl border border-white/10`}
    >
      <div className={`w-8 h-8 border-4 border-t-transparent ${style.spinner} rounded-full animate-spin mb-4`}></div>
      <div className={`text-sm ${style.text} animate-pulse`}>{message}</div>
    </motion.div>
  );
}

// 🎨 Skeleton Loading Components
export function OOFCardSkeleton() {
  return (
    <div className="bg-gradient-to-br from-purple-900/20 to-purple-800/20 rounded-xl border border-purple-500/20 p-6 animate-pulse">
      <div className="flex items-center justify-between mb-4">
        <div className="h-6 bg-purple-500/20 rounded w-1/3"></div>
        <div className="h-8 bg-purple-500/20 rounded w-16"></div>
      </div>
      <div className="space-y-3">
        <div className="h-4 bg-purple-500/20 rounded w-full"></div>
        <div className="h-4 bg-purple-500/20 rounded w-2/3"></div>
        <div className="h-32 bg-purple-500/20 rounded w-full"></div>
      </div>
      <div className="flex gap-2 mt-4">
        <div className="h-8 bg-purple-500/20 rounded w-20"></div>
        <div className="h-8 bg-purple-500/20 rounded w-20"></div>
      </div>
    </div>
  );
}

export function OOFTableSkeleton() {
  return (
    <div className="space-y-3">
      {[...Array(5)].map((_, i) => (
        <div key={i} className="flex items-center gap-4 p-4 bg-purple-900/20 rounded-lg animate-pulse">
          <div className="w-10 h-10 bg-purple-500/20 rounded-full"></div>
          <div className="flex-1 space-y-2">
            <div className="h-4 bg-purple-500/20 rounded w-1/4"></div>
            <div className="h-3 bg-purple-500/20 rounded w-1/2"></div>
          </div>
          <div className="h-8 bg-purple-500/20 rounded w-20"></div>
        </div>
      ))}
    </div>
  );
}

// 🧠 Lazy Loading Wrapper with Error Boundaries
interface LazyComponentProps {
  fallback?: ReactNode;
  errorFallback?: ReactNode;
  retryAttempts?: number;
}

export function withLazyLoading<P extends object>(
  importFn: () => Promise<{ default: ComponentType<P> }>,
  options: LazyComponentProps = {}
) {
  const {
    fallback = <OOFLoadingSpinner />,
    errorFallback = (
      <div className="flex flex-col items-center justify-center min-h-[200px] bg-red-900/20 rounded-xl border border-red-500/20">
        <div className="text-red-400 text-lg mb-2">😵 Component failed to load</div>
        <div className="text-red-300 text-sm mb-4">This is awkward... like selling before a pump</div>
        <button
          onClick={() => window.location.reload()}
          className="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg text-sm"
        >
          Retry Loading
        </button>
      </div>
    ),
    retryAttempts = 3
  } = options;

  const LazyComponent = lazy(() => {
    let attempts = 0;
    const loadWithRetry = async (): Promise<{ default: ComponentType<P> }> => {
      try {
        return await importFn();
      } catch (error) {
        attempts++;
        if (attempts < retryAttempts) {
          // Exponential backoff
          await new Promise(resolve => setTimeout(resolve, Math.pow(2, attempts) * 1000));
          return loadWithRetry();
        }
        throw error;
      }
    };
    return loadWithRetry();
  });

  return function WrappedComponent(props: P) {
    return (
      <Suspense fallback={fallback}>
        <ErrorBoundary fallback={errorFallback}>
          <LazyComponent {...props} />
        </ErrorBoundary>
      </Suspense>
    );
  };
}

// 🛡️ Error Boundary Component
class ErrorBoundary extends React.Component<
  { children: ReactNode; fallback: ReactNode },
  { hasError: boolean; error: Error | null }
> {
  constructor(props: { children: ReactNode; fallback: ReactNode }) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error('OOF Component Error:', error, errorInfo);

    // Track error in analytics
    if (window.gtag) {
      window.gtag('event', 'exception', {
        description: error.message,
        fatal: false
      });
    }
  }

  render() {
    if (this.state.hasError) {
      return this.props.fallback;
    }

    return this.props.children;
  }
}

// 📦 Optimized Route Components with Code Splitting
export const LazyLanding = withLazyLoading(
  () => import('@/pages/Landing'),
  {
    fallback: <OOFLoadingSpinner message="Loading the landing page..." />,
    retryAttempts: 3
  }
);

export const LazyDashboard = withLazyLoading(
  () => import('@/pages/Dashboard'),
  {
    fallback: <OOFLoadingSpinner message="Preparing your dashboard..." />,
    retryAttempts: 3
  }
);

export const LazyOOFMoments = withLazyLoading(
  () => import('@/pages/OOFMoments'),
  {
    fallback: <OOFLoadingSpinner message="Loading AI-powered OOF moments..." emotionalMode="viral" />,
    retryAttempts: 2 // AI features might fail more often
  }
);

export const LazyWalletAnalyzer = withLazyLoading(
  () => import('@/pages/WalletAnalyzer'),
  {
    fallback: <OOFLoadingSpinner message="Preparing wallet analysis tools..." />,
    retryAttempts: 3
  }
);

export const LazyOOFDetectiveAdvanced = withLazyLoading(
  () => import('@/pages/OOFDetectiveAdvanced'),
  {
    fallback: <OOFLoadingSpinner message="Loading advanced AI detective..." emotionalMode="viral" />,
    retryAttempts: 2
  }
);

export const LazyOOFBattleRoyale = withLazyLoading(
  () => import('@/pages/OOFBattleRoyale'),
  {
    fallback: <OOFLoadingSpinner message="Entering the battle arena..." emotionalMode="viral" />,
    retryAttempts: 3
  }
);

export const LazyOOFStaking = withLazyLoading(
  () => import('@/pages/OOFStaking'),
  {
    fallback: <OOFLoadingSpinner message="Loading DeFi staking platform..." />,
    retryAttempts: 3
  }
);

export const LazyOOFSocial = withLazyLoading(
  () => import('@/pages/OOFSocial'),
  {
    fallback: <OOFLoadingSpinner message="Connecting to the OOF community..." emotionalMode="therapeutic" />,
    retryAttempts: 3
  }
);

export const LazyOOFCampaigns = withLazyLoading(
  () => import('@/pages/OOFsCampaigns'),
  {
    fallback: <OOFLoadingSpinner message="Loading viral campaigns..." emotionalMode="viral" />,
    retryAttempts: 2
  }
);

// 🎮 Gaming and Interactive Components
export const LazyTokenExplorer = withLazyLoading(() => import('@/pages/TokenExplorer'));
export const LazyTradersArena = withLazyLoading(() => import('@/pages/TradersArena'));
export const LazyTimeMachine = withLazyLoading(() => import('@/pages/TimeMachine'));
export const LazyOOFDetective = withLazyLoading(() => import('@/pages/OOFDetective'));
export const LazyOOFOrigins = withLazyLoading(() => import('@/pages/OOFOrigins'));
export const LazyOOFLegends = withLazyLoading(() => import('@/pages/OOFLegends'));
export const LazyOOFRealityBender = withLazyLoading(() => import('@/pages/OOFRealityBender'));
export const LazyOOFAirdrop = withLazyLoading(() => import('@/pages/OOFAirdrop'));
export const LazyOOFMultiverse = withLazyLoading(() => import('@/pages/OOFMultiverse'));
export const LazyProfile = withLazyLoading(() => import('@/pages/Profile'));
export const LazySlots = withLazyLoading(() => import('@/pages/Slots'));

// 📄 Static/Info Pages (Lower Priority)
export const LazyDocumentation = withLazyLoading(() => import('@/pages/Documentation'));
export const LazyWhitepaper = withLazyLoading(() => import('@/pages/Whitepaper'));
export const LazyPartnerships = withLazyLoading(() => import('@/pages/Partnerships'));
export const LazySupport = withLazyLoading(() => import('@/pages/Support'));
export const LazyAPI = withLazyLoading(() => import('@/pages/API'));
export const LazyAchievements = withLazyLoading(() => import('@/pages/Achievements'));

// 🎯 Image Optimization Hook
export function useImageOptimization() {
  const [imageLoadingStates, setImageLoadingStates] = useState<Record<string, boolean>>({});

  const handleImageLoad = (src: string) => {
    setImageLoadingStates(prev => ({ ...prev, [src]: true }));
  };

  const ImageWithLoading = ({
    src,
    alt,
    className,
    fallback
  }: {
    src: string;
    alt: string;
    className?: string;
    fallback?: ReactNode;
  }) => {
    const isLoaded = imageLoadingStates[src];

    return (
      <div className={`relative ${className}`}>
        {!isLoaded && (
          <div className="absolute inset-0 bg-purple-900/20 animate-pulse rounded" />
        )}
        <img
          src={src}
          alt={alt}
          onLoad={() => handleImageLoad(src)}
          onError={() => console.warn(`Failed to load image: ${src}`)}
          className={`transition-opacity duration-300 ${isLoaded ? 'opacity-100' : 'opacity-0'} ${className}`}
          loading="lazy"
        />
        {!isLoaded && fallback}
      </div>
    );
  };

  return { ImageWithLoading };
}

// ⚡ Performance Monitoring Hook
export function usePerformanceMonitoring() {
  useEffect(() => {
    // Monitor Core Web Vitals
    if ('web-vital' in window) {
      // @ts-ignore
      import('web-vitals').then(({ getCLS, getFID, getFCP, getLCP, getTTFB }) => {
        getCLS(console.log);
        getFID(console.log);
        getFCP(console.log);
        getLCP(console.log);
        getTTFB(console.log);
      });
    }

    // Monitor bundle sizes
    if (process.env.NODE_ENV === 'development') {
      console.log('📦 OOF Performance Monitoring Active');

      // Log current bundle information
      const navigationEntries = performance.getEntriesByType('navigation');
      if (navigationEntries.length > 0) {
        const nav = navigationEntries[0] as PerformanceNavigationTiming;
        console.log('🚀 Page Load Performance:', {
          domContentLoaded: `${nav.domContentLoadedEventEnd - nav.navigationStart}ms`,
          fullLoad: `${nav.loadEventEnd - nav.navigationStart}ms`,
          firstContentfulPaint: `${nav.domContentLoadedEventStart - nav.navigationStart}ms`
        });
      }
    }
  }, []);

  const trackComponentMount = (componentName: string) => {
    const startTime = performance.now();

    return () => {
      const endTime = performance.now();
      const renderTime = endTime - startTime;

      if (renderTime > 100) { // Log slow renders
        console.warn(`🐌 Slow render detected: ${componentName} took ${renderTime.toFixed(2)}ms`);
      }

      // Track in analytics
      if (window.gtag) {
        window.gtag('event', 'component_render_time', {
          component_name: componentName,
          render_time: renderTime
        });
      }
    };
  };

  return { trackComponentMount };
}

// 🎨 Animation Performance Optimization
export function useOptimizedAnimations() {
  const [reducedMotion, setReducedMotion] = useState(false);

  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
    setReducedMotion(mediaQuery.matches);

    const handleChange = (e: MediaQueryListEvent) => setReducedMotion(e.matches);
    mediaQuery.addEventListener('change', handleChange);

    return () => mediaQuery.removeEventListener('change', handleChange);
  }, []);

  const getOptimizedVariants = (originalVariants: any) => {
    if (reducedMotion) {
      // Return simplified variants with no complex animations
      return {
        initial: { opacity: 0 },
        animate: { opacity: 1 },
        exit: { opacity: 0 }
      };
    }
    return originalVariants;
  };

  const OptimizedMotion = ({ children, variants, ...props }: any) => {
    const optimizedVariants = getOptimizedVariants(variants);

    return (
      <motion.div
        variants={optimizedVariants}
        {...props}
      >
        {children}
      </motion.div>
    );
  };

  return { OptimizedMotion, reducedMotion };
}

// 🚀 Bundle Analysis Utilities (Development Only)
export const bundleAnalysis = {
  logBundleSize: () => {
    if (process.env.NODE_ENV === 'development') {
      // This would integrate with webpack-bundle-analyzer in a real setup
      console.log('📊 Bundle Analysis Available in Production Build');
    }
  },

  trackAsyncChunkLoad: (chunkName: string) => {
    if (process.env.NODE_ENV === 'development') {
      console.log(`📦 Loading async chunk: ${chunkName}`);
    }

    const startTime = performance.now();
    return () => {
      const loadTime = performance.now() - startTime;
      console.log(`✅ Chunk ${chunkName} loaded in ${loadTime.toFixed(2)}ms`);
    };
  }
};
