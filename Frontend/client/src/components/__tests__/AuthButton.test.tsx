// 🧪 AuthButton Component Tests

import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import AuthButton from '../AuthButton';
import { OOFAuthProvider } from '@/providers/AuthProvider';
import { OOFThemeProvider } from '@/providers/ThemeProvider';

// Test wrapper with providers
const TestWrapper = ({ children }: { children: React.ReactNode }) => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false }
    }
  });

  return (
    <QueryClientProvider client={queryClient}>
      <OOFThemeProvider>
        <OOFAuthProvider>
          {children}
        </OOFAuthProvider>
      </OOFThemeProvider>
    </QueryClientProvider>
  );
};

describe('AuthButton', () => {
  it('renders login button when user is not authenticated', () => {
    // Mock unauthenticated state
    vi.mocked(require('@/providers/AuthProvider').useOOFAuth).mockReturnValue({
      user: null,
      primaryWallet: null,
      isAuthenticated: false,
      isLoading: false,
      connect: vi.fn(),
      disconnect: vi.fn()
    });

    render(
      <TestWrapper>
        <AuthButton />
      </TestWrapper>
    );

    expect(screen.getByText('Login / Signup')).toBeInTheDocument();
    expect(screen.getByRole('button')).toHaveClass('bg-gradient-to-r');
  });

  it('renders user info and logout button when authenticated', () => {
    // Mock authenticated state
    vi.mocked(require('@/providers/AuthProvider').useOOFAuth).mockReturnValue({
      user: {
        id: 'test-user',
        email: 'test@oof-platform.com',
        firstName: 'Test',
        walletAddress: '9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM'
      },
      primaryWallet: {
        address: '9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM'
      },
      isAuthenticated: true,
      isLoading: false,
      connect: vi.fn(),
      disconnect: vi.fn()
    });

    render(
      <TestWrapper>
        <AuthButton />
      </TestWrapper>
    );

    expect(screen.getByText('test@oof-platform.com')).toBeInTheDocument();
    expect(screen.getByText(/9WzD.*AWWM/)).toBeInTheDocument();
    expect(screen.getByText('Logout')).toBeInTheDocument();
  });

  it('calls connect function when login button is clicked', async () => {
    const mockConnect = vi.fn();

    vi.mocked(require('@/providers/AuthProvider').useOOFAuth).mockReturnValue({
      user: null,
      primaryWallet: null,
      isAuthenticated: false,
      isLoading: false,
      connect: mockConnect,
      disconnect: vi.fn()
    });

    render(
      <TestWrapper>
        <AuthButton />
      </TestWrapper>
    );

    const loginButton = screen.getByText('Login / Signup');
    fireEvent.click(loginButton);

    await waitFor(() => {
      expect(mockConnect).toHaveBeenCalledOnce();
    });
  });

  it('calls disconnect function when logout button is clicked', async () => {
    const mockDisconnect = vi.fn();

    vi.mocked(require('@/providers/AuthProvider').useOOFAuth).mockReturnValue({
      user: {
        id: 'test-user',
        email: 'test@oof-platform.com',
        firstName: 'Test'
      },
      primaryWallet: { address: '9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM' },
      isAuthenticated: true,
      isLoading: false,
      connect: vi.fn(),
      disconnect: mockDisconnect
    });

    render(
      <TestWrapper>
        <AuthButton />
      </TestWrapper>
    );

    const logoutButton = screen.getByText('Logout');
    fireEvent.click(logoutButton);

    await waitFor(() => {
      expect(mockDisconnect).toHaveBeenCalledOnce();
    });
  });

  it('shows loading state when authentication is in progress', () => {
    vi.mocked(require('@/providers/AuthProvider').useOOFAuth).mockReturnValue({
      user: null,
      primaryWallet: null,
      isAuthenticated: false,
      isLoading: true,
      connect: vi.fn(),
      disconnect: vi.fn()
    });

    render(
      <TestWrapper>
        <AuthButton />
      </TestWrapper>
    );

    expect(screen.getByText('Connecting...')).toBeInTheDocument();
    expect(screen.getByRole('button')).toBeDisabled();
  });

  it('displays wallet address with proper truncation', () => {
    const longWalletAddress = '9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM';

    vi.mocked(require('@/providers/AuthProvider').useOOFAuth).mockReturnValue({
      user: {
        id: 'test-user',
        email: 'test@oof-platform.com',
        firstName: 'Test'
      },
      primaryWallet: { address: longWalletAddress },
      isAuthenticated: true,
      isLoading: false,
      connect: vi.fn(),
      disconnect: vi.fn()
    });

    render(
      <TestWrapper>
        <AuthButton />
      </TestWrapper>
    );

    // Should show truncated address (first 4 + last 4 characters)
    expect(screen.getByText('9WzD...AWWM')).toBeInTheDocument();
  });

  it('has proper accessibility attributes', () => {
    vi.mocked(require('@/providers/AuthProvider').useOOFAuth).mockReturnValue({
      user: null,
      primaryWallet: null,
      isAuthenticated: false,
      isLoading: false,
      connect: vi.fn(),
      disconnect: vi.fn()
    });

    render(
      <TestWrapper>
        <AuthButton />
      </TestWrapper>
    );

    const button = screen.getByRole('button');
    expect(button).toBeInTheDocument();
    expect(button).not.toBeDisabled();
  });
});
