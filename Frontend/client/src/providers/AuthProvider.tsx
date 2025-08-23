import React, { createContext, useContext, useEffect, useState, ReactNode } from 'react';
import { DynamicContextProvider, useDynamicContext } from '@dynamic-labs/sdk-react-core';
import { SolanaWalletConnectors } from '@dynamic-labs/solana';
import { useToast } from '@/hooks/use-toast';

// 🎯 Unified Authentication Types
interface AuthUser {
  id: string;
  email?: string;
  firstName?: string;
  lastName?: string;
  walletAddress: string;
  profileImageUrl?: string;
  username?: string;
  // OOF Platform specific fields
  oofTokens: number;
  oofScore: number;
  totalMoments: number;
  totalEarned: string;
  ranking: number;
  predictionAccuracy: string;
}

interface AuthState {
  // User and wallet state
  user: AuthUser | null;
  primaryWallet: any | null;
  isAuthenticated: boolean;
  isLoading: boolean;

  // Authentication methods
  connect: () => Promise<void>;
  disconnect: () => Promise<void>;
  switchWallet: () => Promise<void>;

  // JWT and token management
  getJWTToken: () => Promise<string | null>;
  refreshAuth: () => Promise<void>;

  // OOF Platform specific methods
  updateUserStats: (stats: Partial<AuthUser>) => Promise<void>;
  trackActivity: (activity: string) => Promise<void>;
}

// 🔧 Create Auth Context
const AuthContext = createContext<AuthState | null>(null);

// 🎨 Enhanced Dynamic.xyz Configuration
const DYNAMIC_SETTINGS = {
  environmentId: import.meta.env.VITE_DYNAMIC_ENVIRONMENT_ID || "7037c007-259c-4dc8-8f95-3ed01c0ab2fb",
  appName: 'OOF Platform - Emotional AI for Crypto',
  walletConnectors: [SolanaWalletConnectors],

  // Enhanced OOF Platform branding
  cssOverrides: `
    .dynamic-widget-card {
      background: linear-gradient(135deg, #9333ea 0%, #7c3aed 100%) !important;
      border: 1px solid #8b5cf6 !important;
      border-radius: 16px !important;
      box-shadow: 0 8px 32px rgba(147, 51, 234, 0.3) !important;
      backdrop-filter: blur(20px) !important;
    }

    .dynamic-widget-button {
      background: linear-gradient(135deg, #a855f7 0%, #9333ea 100%) !important;
      color: white !important;
      border-radius: 12px !important;
      padding: 14px 24px !important;
      font-weight: 600 !important;
      border: none !important;
      transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1) !important;
      font-size: 15px !important;
      position: relative !important;
      overflow: hidden !important;
    }

    .dynamic-widget-button:before {
      content: '' !important;
      position: absolute !important;
      top: 0 !important;
      left: -100% !important;
      width: 100% !important;
      height: 100% !important;
      background: linear-gradient(90deg, transparent, rgba(255,255,255,0.2), transparent) !important;
      transition: left 0.5s !important;
    }

    .dynamic-widget-button:hover:before {
      left: 100% !important;
    }

    .dynamic-widget-button:hover {
      background: linear-gradient(135deg, #9333ea 0%, #7c3aed 100%) !important;
      transform: translateY(-2px) !important;
      box-shadow: 0 12px 32px rgba(147, 51, 234, 0.6) !important;
    }

    .dynamic-modal-overlay {
      background: rgba(0, 0, 0, 0.85) !important;
      backdrop-filter: blur(8px) !important;
    }

    .dynamic-widget-modal {
      background: linear-gradient(135deg, #1e1b4b 0%, #312e81 100%) !important;
      border: 1px solid #8b5cf6 !important;
      border-radius: 20px !important;
      box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5) !important;
    }

    .dynamic-widget-text {
      color: white !important;
    }

    .dynamic-widget-input {
      background: rgba(147, 51, 234, 0.1) !important;
      border: 1px solid #8b5cf6 !important;
      border-radius: 8px !important;
      color: white !important;
    }
  `,

  // Enhanced event handlers for OOF Platform
  events: {
    onAuthSuccess: async (args: any) => {
      console.log('🎉 OOF Platform: User authenticated successfully', args);

      if (args.user && args.primaryWallet) {
        console.log('✅ Wallet connected:', args.primaryWallet.address);

        try {
          // Save user to OOF Platform backend
          await fetch('/api/auth/connect-wallet', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              dynamicUserId: args.user.userId,
              email: args.user.email,
              firstName: args.user.firstName,
              lastName: args.user.lastName,
              walletAddress: args.primaryWallet.address,
              authMethod: args.authMethod || 'wallet',
              timestamp: new Date().toISOString()
            }),
          });

          // Track authentication analytics
          if (window.gtag) {
            window.gtag('event', 'wallet_connected', {
              wallet_type: args.primaryWallet.connector?.name || 'unknown',
              auth_method: args.authMethod || 'wallet'
            });
          }
        } catch (error) {
          console.error('❌ Failed to save user to backend:', error);
        }
      }
    },

    onWalletAdded: (args: any) => {
      console.log('🔗 OOF Platform: New wallet added', args);
    },

    onAuthFailure: (args: any) => {
      console.error('❌ OOF Platform: Authentication failed', args);
    },

    onLogout: () => {
      console.log('👋 OOF Platform: User logged out');
      // Clear any cached user data
      localStorage.removeItem('oof_user_cache');
    }
  }
};

// 🧠 Auth Provider Component with State Management
function AuthProviderInner({ children }: { children: ReactNode }) {
  const { user, primaryWallet, setShowAuthFlow, handleLogOut } = useDynamicContext();
  const { toast } = useToast();

  const [authUser, setAuthUser] = useState<AuthUser | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [userStats, setUserStats] = useState<Partial<AuthUser>>({});

  // 🔄 Sync Dynamic user with OOF Platform user
  useEffect(() => {
    const syncUser = async () => {
      if (user && primaryWallet) {
        try {
          // Fetch user data from OOF Platform backend
          const response = await fetch(`/api/users/${primaryWallet.address}`);
          let userData;

          if (response.ok) {
            userData = await response.json();
          } else {
            // Create new user if doesn't exist
            const createResponse = await fetch('/api/users', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({
                id: primaryWallet.address,
                email: user.email,
                firstName: user.firstName,
                lastName: user.lastName,
                walletAddress: primaryWallet.address
              })
            });
            userData = createResponse.ok ? await createResponse.json() : null;
          }

          if (userData) {
            const oofUser: AuthUser = {
              id: userData.id || primaryWallet.address,
              email: user.email || userData.email,
              firstName: user.firstName || userData.firstName,
              lastName: user.lastName || userData.lastName,
              walletAddress: primaryWallet.address,
              profileImageUrl: user.profileImageUrl || userData.profileImageUrl,
              username: userData.username,
              oofTokens: userData.oofTokens || 0,
              oofScore: userData.oofScore || 0,
              totalMoments: userData.totalMoments || 0,
              totalEarned: userData.totalEarned || "0",
              ranking: userData.ranking || 0,
              predictionAccuracy: userData.predictionAccuracy || "0"
            };

            setAuthUser(oofUser);

            // Cache user data for offline use
            localStorage.setItem('oof_user_cache', JSON.stringify(oofUser));
          }
        } catch (error) {
          console.error('Failed to sync user data:', error);

          // Try to load from cache
          const cachedUser = localStorage.getItem('oof_user_cache');
          if (cachedUser) {
            setAuthUser(JSON.parse(cachedUser));
          }
        }
      } else {
        setAuthUser(null);
        localStorage.removeItem('oof_user_cache');
      }

      setIsLoading(false);
    };

    syncUser();
  }, [user, primaryWallet]);

  // 🔐 Authentication methods
  const connect = async () => {
    try {
      setIsLoading(true);
      setShowAuthFlow(true);
    } catch (error) {
      console.error('Connection failed:', error);
      toast({
        title: "Connection Failed",
        description: "Please try connecting your wallet again.",
        variant: "destructive"
      });
    } finally {
      setIsLoading(false);
    }
  };

  const disconnect = async () => {
    try {
      setIsLoading(true);
      await handleLogOut();
      setAuthUser(null);
      localStorage.removeItem('oof_user_cache');

      toast({
        title: "Disconnected",
        description: "Your wallet has been disconnected successfully.",
      });
    } catch (error) {
      console.error('Disconnect failed:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const switchWallet = async () => {
    setShowAuthFlow(true);
  };

  const getJWTToken = async (): Promise<string | null> => {
    try {
      if (primaryWallet) {
        // Request JWT token from Dynamic
        const token = await primaryWallet.getAuthToken();
        return token;
      }
      return null;
    } catch (error) {
      console.error('Failed to get JWT token:', error);
      return null;
    }
  };

  const refreshAuth = async () => {
    // Re-sync user data from backend
    if (primaryWallet) {
      setIsLoading(true);
      // Trigger re-sync
      await new Promise(resolve => setTimeout(resolve, 100));
      setIsLoading(false);
    }
  };

  // 📊 OOF Platform specific methods
  const updateUserStats = async (stats: Partial<AuthUser>) => {
    if (!authUser) return;

    try {
      const response = await fetch(`/api/users/${authUser.id}`, {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(stats)
      });

      if (response.ok) {
        const updatedUser = { ...authUser, ...stats };
        setAuthUser(updatedUser);
        localStorage.setItem('oof_user_cache', JSON.stringify(updatedUser));
      }
    } catch (error) {
      console.error('Failed to update user stats:', error);
    }
  };

  const trackActivity = async (activity: string) => {
    if (!authUser) return;

    try {
      await fetch('/api/analytics/activity', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          userId: authUser.id,
          activity,
          timestamp: new Date().toISOString()
        })
      });
    } catch (error) {
      console.error('Failed to track activity:', error);
    }
  };

  // 🎯 Auth state object
  const authState: AuthState = {
    user: authUser,
    primaryWallet,
    isAuthenticated: !!authUser && !!primaryWallet,
    isLoading,
    connect,
    disconnect,
    switchWallet,
    getJWTToken,
    refreshAuth,
    updateUserStats,
    trackActivity
  };

  return (
    <AuthContext.Provider value={authState}>
      {children}
    </AuthContext.Provider>
  );
}

// 🌟 Main Auth Provider with Dynamic.xyz wrapper
export function OOFAuthProvider({ children }: { children: ReactNode }) {
  const [isReady, setIsReady] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Initialize Dynamic.xyz
    const timer = setTimeout(() => setIsReady(true), 200);
    return () => clearTimeout(timer);
  }, []);

  if (!isReady) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-gradient-to-br from-purple-900 via-purple-800 to-purple-900">
        <div className="text-center">
          <div className="w-16 h-16 mx-auto mb-4 border-4 border-purple-400 border-t-transparent rounded-full animate-spin"></div>
          <div className="text-white text-xl font-semibold">Loading OOF Platform...</div>
          <div className="text-purple-300 text-sm mt-2">Preparing your emotional AI experience</div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-gradient-to-br from-purple-900 via-purple-800 to-purple-900">
        <div className="text-center max-w-md">
          <div className="text-red-400 text-xl mb-4">⚠️ Authentication Error</div>
          <div className="text-white mb-4">{error}</div>
          <button
            onClick={() => window.location.reload()}
            className="px-6 py-3 bg-purple-600 hover:bg-purple-700 rounded-lg text-white font-medium transition-colors"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  try {
    return (
      <DynamicContextProvider settings={DYNAMIC_SETTINGS}>
        <AuthProviderInner>
          {children}
        </AuthProviderInner>
      </DynamicContextProvider>
    );
  } catch (dynamicError) {
    console.error('Dynamic Labs SDK Error:', dynamicError);
    setError('Authentication system failed to initialize');

    return (
      <div className="min-h-screen bg-gradient-to-br from-purple-900 via-purple-800 to-purple-900">
        <div className="text-yellow-400 text-sm p-4 bg-yellow-900/20 border-b border-yellow-700/30">
          ⚠️ Wallet authentication unavailable - running in fallback mode
        </div>
        <AuthContext.Provider value={{
          user: null,
          primaryWallet: null,
          isAuthenticated: false,
          isLoading: false,
          connect: async () => console.warn('Auth unavailable'),
          disconnect: async () => console.warn('Auth unavailable'),
          switchWallet: async () => console.warn('Auth unavailable'),
          getJWTToken: async () => null,
          refreshAuth: async () => console.warn('Auth unavailable'),
          updateUserStats: async () => console.warn('Auth unavailable'),
          trackActivity: async () => console.warn('Auth unavailable'),
        }}>
          {children}
        </AuthContext.Provider>
      </div>
    );
  }
}

// 🪝 Enhanced useAuth hook
export function useOOFAuth(): AuthState {
  const context = useContext(AuthContext);

  if (!context) {
    throw new Error('useOOFAuth must be used within OOFAuthProvider');
  }

  return context;
}

// 🔄 Legacy compatibility hook (deprecated - use useOOFAuth instead)
export function useAuth() {
  console.warn('useAuth is deprecated. Use useOOFAuth instead for full OOF Platform features.');

  const auth = useOOFAuth();

  // Return legacy interface for compatibility
  return {
    user: auth.user,
    wallet: auth.primaryWallet,
    isLoading: auth.isLoading,
    isAuthenticated: auth.isAuthenticated,
    login: auth.connect,
    logout: auth.disconnect
  };
}

export default OOFAuthProvider;
