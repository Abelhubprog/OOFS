// 🚨 DEPRECATED: Use useOOFAuth from @/providers/AuthProvider instead
// This hook is maintained for backward compatibility only

import { useOOFAuth } from '@/providers/AuthProvider';

/**
 * @deprecated Use useOOFAuth from @/providers/AuthProvider instead
 * This hook will be removed in a future version
 */
export function useAuth() {
  console.warn('⚠️ useAuth is deprecated. Please migrate to useOOFAuth from @/providers/AuthProvider for full OOF Platform features.');

  const auth = useOOFAuth();

  // Return legacy interface for backward compatibility
  return {
    user: auth.user,
    wallet: auth.primaryWallet,
    isLoading: auth.isLoading,
    isAuthenticated: auth.isAuthenticated,
    login: auth.connect,
    logout: auth.disconnect
  };
}
