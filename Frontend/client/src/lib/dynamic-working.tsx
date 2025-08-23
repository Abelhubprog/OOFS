import { DynamicContextProvider } from '@dynamic-labs/sdk-react-core';
import { SolanaWalletConnectors } from '@dynamic-labs/solana';
import { ReactNode, useEffect, useState } from 'react';

const DynamicProvider = ({ children }: { children: ReactNode }) => {
  const [isReady, setIsReady] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Add a delay to ensure the component is ready
    const timer = setTimeout(() => {
      setIsReady(true);
    }, 100);

    return () => clearTimeout(timer);
  }, []);

  if (!isReady) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-gradient-to-br from-purple-900 via-purple-800 to-purple-900">
        <div className="text-white text-xl">Loading OOF Platform...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-gradient-to-br from-purple-900 via-purple-800 to-purple-900">
        <div className="text-red-400 text-xl">Error: {error}</div>
        <div className="text-white">{children}</div>
      </div>
    );
  }

  try {
    return (
      <DynamicContextProvider
        settings={{
          environmentId: "7037c007-259c-4dc8-8f95-3ed01c0ab2fb",
        walletConnectors: [SolanaWalletConnectors],
        appName: 'OOF Platform',
        
        // Enhanced styling for OOF branding
        cssOverrides: `
          .dynamic-widget-card {
            background: linear-gradient(135deg, #9333ea 0%, #7c3aed 100%) !important;
            border: 1px solid #8b5cf6 !important;
            border-radius: 12px !important;
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3) !important;
            backdrop-filter: blur(10px) !important;
          }
          
          .dynamic-widget-button {
            background: linear-gradient(135deg, #a855f7 0%, #9333ea 100%) !important;
            color: white !important;
            border-radius: 8px !important;
            padding: 12px 20px !important;
            font-weight: 600 !important;
            border: none !important;
            transition: all 0.3s ease !important;
            font-size: 14px !important;
          }
          
          .dynamic-widget-button:hover {
            background: linear-gradient(135deg, #9333ea 0%, #7c3aed 100%) !important;
            transform: translateY(-2px) !important;
            box-shadow: 0 8px 24px rgba(147, 51, 234, 0.5) !important;
          }
          
          .dynamic-modal-overlay {
            background: rgba(0, 0, 0, 0.8) !important;
            backdrop-filter: blur(4px) !important;
          }
          
          .dynamic-widget-text {
            color: white !important;
          }
          
          .dynamic-widget-modal {
            background: linear-gradient(135deg, #1e1b4b 0%, #312e81 100%) !important;
            border: 1px solid #8b5cf6 !important;
            border-radius: 16px !important;
          }
        `,
        
        // Event handlers for wallet lifecycle
        events: {
          onAuthSuccess: (args: any) => {
            console.log('OOF Platform: User authenticated successfully', args);
            if (args.user && args.primaryWallet) {
              console.log('Wallet connected:', args.primaryWallet.address);
              
              // Save wallet to backend
              fetch('/api/save-wallet', {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify({
                  email: args.user.email,
                  walletAddress: args.primaryWallet.address,
                  userId: args.user.userId || args.primaryWallet.address
                }),
              }).catch(console.error);
            }
          },
          
          onWalletAdded: (args: any) => {
            console.log('OOF Platform: New wallet added', args);
          }
        }
      }}
    >
      {children}
    </DynamicContextProvider>
  );
  } catch (dynamicError) {
    console.error('Dynamic Labs SDK Error:', dynamicError);
    setError('Dynamic Labs SDK failed to load');
    
    // Fallback: render children without Dynamic provider
    return (
      <div className="min-h-screen bg-gradient-to-br from-purple-900 via-purple-800 to-purple-900">
        <div className="text-yellow-400 text-sm p-4">
          ⚠️ Wallet provider unavailable - running in fallback mode
        </div>
        {children}
      </div>
    );
  }
};

export default DynamicProvider;