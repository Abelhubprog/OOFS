// 🧪 Test Setup for OOF Platform Frontend

import '@testing-library/jest-dom';
import { expect, afterEach, vi, beforeAll, afterAll } from 'vitest';
import { cleanup } from '@testing-library/react';
import { server } from './mocks/server';

// Mock environment variables
vi.mock('process', () => ({
  env: {
    NODE_ENV: 'test',
    VITE_DYNAMIC_ENVIRONMENT_ID: 'test-env-id',
    VITE_API_URL: 'http://localhost:3000'
  }
}));

// Mock Dynamic.xyz
vi.mock('@dynamic-labs/sdk-react-core', () => ({
  DynamicContextProvider: ({ children }: { children: React.ReactNode }) => children,
  DynamicWidget: () => <div data-testid="dynamic-widget">Mock Dynamic Widget</div>,
  useDynamicContext: () => ({
    user: {
      id: 'test-user-id',
      email: 'test@oof-platform.com',
      firstName: 'Test',
      walletAddress: '9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM'
    },
    primaryWallet: {
      address: '9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM'
    },
    isAuthenticated: true,
    setShowAuthFlow: vi.fn(),
    handleLogOut: vi.fn()
  })
}));

// Mock Solana Web3.js
vi.mock('@solana/web3.js', () => ({
  Connection: vi.fn().mockImplementation(() => ({
    getBalance: vi.fn().mockResolvedValue(1000000000), // 1 SOL
    getSignaturesForAddress: vi.fn().mockResolvedValue([]),
    getParsedTransaction: vi.fn().mockResolvedValue(null)
  })),
  PublicKey: vi.fn().mockImplementation((key: string) => ({
    toString: () => key,
    toBase58: () => key
  })),
  LAMPORTS_PER_SOL: 1000000000
}));

// Mock Framer Motion
vi.mock('framer-motion', () => ({
  motion: {
    div: 'div',
    button: 'button',
    span: 'span',
    h1: 'h1',
    h2: 'h2',
    h3: 'h3'
  },
  AnimatePresence: ({ children }: { children: React.ReactNode }) => children
}));

// Mock React Query
vi.mock('@tanstack/react-query', () => ({
  QueryClient: vi.fn().mockImplementation(() => ({
    invalidateQueries: vi.fn(),
    setQueryData: vi.fn(),
    getQueryData: vi.fn()
  })),
  QueryClientProvider: ({ children }: { children: React.ReactNode }) => children,
  useQuery: vi.fn(() => ({
    data: null,
    isLoading: false,
    error: null,
    refetch: vi.fn()
  })),
  useMutation: vi.fn(() => ({
    mutate: vi.fn(),
    isPending: false,
    error: null,
    data: null
  }))
}));

// Mock Wouter router
vi.mock('wouter', () => ({
  Link: ({ href, children, ...props }: any) => (
    <a href={href} {...props}>{children}</a>
  ),
  useLocation: () => ['/', vi.fn()],
  Switch: ({ children }: { children: React.ReactNode }) => children,
  Route: ({ component: Component, ...props }: any) => Component ? <Component {...props} /> : null
}));

// Mock localStorage
const localStorageMock = {
  getItem: vi.fn(),
  setItem: vi.fn(),
  removeItem: vi.fn(),
  clear: vi.fn(),
};

Object.defineProperty(window, 'localStorage', {
  value: localStorageMock
});

// Mock window.matchMedia
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation(query => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

// Mock IntersectionObserver
global.IntersectionObserver = vi.fn().mockImplementation(() => ({
  observe: vi.fn(),
  unobserve: vi.fn(),
  disconnect: vi.fn()
}));

// Mock ResizeObserver
global.ResizeObserver = vi.fn().mockImplementation(() => ({
  observe: vi.fn(),
  unobserve: vi.fn(),
  disconnect: vi.fn()
}));

// Setup MSW
beforeAll(() => server.listen({ onUnhandledRequest: 'error' }));
afterEach(() => {
  server.resetHandlers();
  cleanup();
  vi.clearAllMocks();
});
afterAll(() => server.close());

// Custom test utilities
export const createMockUser = (overrides = {}) => ({
  id: 'test-user-id',
  email: 'test@oof-platform.com',
  firstName: 'Test',
  lastName: 'User',
  walletAddress: '9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM',
  oofTokens: 1000,
  ...overrides
});

export const createMockToken = (overrides = {}) => ({
  mint: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
  name: 'Test Token',
  symbol: 'TEST',
  price: 1.0,
  marketCap: 1000000,
  liquidity: 50000,
  bondingCurveProgress: 75.5,
  createdAt: new Date('2024-01-01'),
  volume24h: 100000,
  priceChange24h: 5.2,
  ...overrides
});

export const createMockOOFMoment = (overrides = {}) => ({
  id: 1,
  title: 'Test OOF Moment',
  description: 'A test moment of regret',
  quote: 'This is a test quote',
  rarity: 'epic' as const,
  momentType: 'paper_hands' as const,
  tokenSymbol: 'TEST',
  tokenAddress: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
  walletAddress: '9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM',
  cardMetadata: {
    background: '#8b5cf6',
    emoji: '💎',
    textColor: '#ffffff',
    accentColor: '#a855f7',
    gradientFrom: 'from-purple-600',
    gradientTo: 'to-purple-800'
  },
  socialStats: {
    upvotes: 42,
    downvotes: 5,
    likes: 38,
    comments: 12,
    shares: 8,
    views: 150
  },
  hashtags: ['#OOF', '#TEST', '#REGRET'],
  isPublic: true,
  createdAt: new Date('2024-01-01'),
  ...overrides
});

// Test assertion extensions
expect.extend({
  toBeAccessible: (received) => {
    // Custom accessibility matcher
    const hasAriaLabel = received.getAttribute('aria-label');
    const hasRole = received.getAttribute('role');

    return {
      message: () => 'Expected element to be accessible',
      pass: !!(hasAriaLabel || hasRole)
    };
  }
});
