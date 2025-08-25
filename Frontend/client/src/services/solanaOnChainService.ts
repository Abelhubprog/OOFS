import { Connection, PublicKey, Transaction, SystemProgram, LAMPORTS_PER_SOL } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, createTransferInstruction, getAssociatedTokenAddress } from '@solana/spl-token';
import { apiRequest } from '@/lib/queryClient';

interface TransactionResult {
  success: boolean;
  signature?: string;
  error?: string;
}

interface TokenLaunchParams {
  name: string;
  symbol: string;
  description: string;
  imageUrl?: string;
  website?: string;
  twitter?: string;
  telegram?: string;
  initialSupply: number;
  creatorWallet: string;
}

class SolanaOnChainService {
  private connection: Connection;

  constructor() {
    // Use backend-configured RPC endpoint
    this.connection = new Connection(
      process.env.VITE_SOLANA_RPC_URL || 'https://api.devnet.solana.com',
      'confirmed'
    );
  }

  async getWalletBalance(walletAddress: string): Promise<number> {
    try {
      // Use new Rust backend API for consistent wallet data
      const response = await fetch(`/v1/wallets/${walletAddress}/balance`, {
        credentials: 'include'
      });

      if (!response.ok) {
        throw new Error('Failed to fetch wallet balance from backend');
      }

      const data = await response.json();
      return data.balance || 0;
    } catch (error) {
      console.error('Error fetching wallet balance:', error);

      // Fallback to direct RPC call
      try {
        const publicKey = new PublicKey(walletAddress);
        const balance = await this.connection.getBalance(publicKey);
        return balance / LAMPORTS_PER_SOL;
      } catch (rpcError) {
        console.error('RPC fallback failed:', rpcError);
        return 0;
      }
    }
  }

  async getTokenBalance(walletAddress: string, tokenMint: string): Promise<number> {
    try {
      // Use new Rust backend API for token balance
      const response = await fetch(`/v1/wallets/${walletAddress}/tokens/${tokenMint}`, {
        credentials: 'include'
      });

      if (!response.ok) {
        throw new Error('Failed to fetch token balance from backend');
      }

      const data = await response.json();
      return data.balance || 0;
    } catch (error) {
      console.error('Error fetching token balance:', error);

      // Fallback to direct RPC call
      try {
        const publicKey = new PublicKey(walletAddress);
        const mintPublicKey = new PublicKey(tokenMint);

        const tokenAccounts = await this.connection.getParsedTokenAccountsByOwner(
          publicKey,
          { mint: mintPublicKey }
        );

        if (tokenAccounts.value.length === 0) {
          return 0;
        }

        const balance = tokenAccounts.value[0].account.data.parsed.info.tokenAmount.uiAmount;
        return balance || 0;
      } catch (rpcError) {
        console.error('RPC fallback failed:', rpcError);
        return 0;
      }
    }
  }

  async launchPumpFunToken(params: TokenLaunchParams): Promise<TransactionResult> {
    try {
      // Use new Rust backend API for token launching
      const response = await apiRequest('POST', '/v1/tokens/launch', params);
      const data = await response.json();

      return {
        success: data.success,
        signature: data.signature,
        error: data.error
      };
    } catch (error) {
      console.error('Error launching token:', error);
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  async buyPumpFunToken(
    tokenMint: string,
    solAmount: number,
    buyerWallet: string
  ): Promise<TransactionResult> {
    try {
      // Use new Rust backend API for token purchases
      const response = await apiRequest('POST', '/v1/tokens/buy', {
        tokenMint,
        solAmount,
        buyerWallet
      });
      const data = await response.json();

      return {
        success: data.success,
        signature: data.signature,
        error: data.error
      };
    } catch (error) {
      console.error('Error buying token:', error);
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  async sellPumpFunToken(
    tokenMint: string,
    tokenAmount: number,
    sellerWallet: string
  ): Promise<TransactionResult> {
    try {
      // Use new Rust backend API for token sales
      const response = await apiRequest('POST', '/v1/tokens/sell', {
        tokenMint,
        tokenAmount,
        sellerWallet
      });
      const data = await response.json();

      return {
        success: data.success,
        signature: data.signature,
        error: data.error
      };
    } catch (error) {
      console.error('Error selling token:', error);
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  async swapTokens(
    fromMint: string,
    toMint: string,
    amount: number,
    walletAddress: string,
    slippage: number = 1
  ): Promise<TransactionResult> {
    try {
      // Use new Rust backend API with Jupiter integration
      const response = await apiRequest('POST', '/v1/swap', {
        fromMint,
        toMint,
        amount,
        walletAddress,
        slippage
      });
      const data = await response.json();

      return {
        success: data.success,
        signature: data.signature,
        error: data.error
      };
    } catch (error) {
      console.error('Error swapping tokens:', error);
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  async getWalletAnalysis(walletAddress: string) {
    try {
      // Use new Rust backend API for wallet analysis
      const response = await fetch(`/v1/wallets/${walletAddress}/summary`, {
        credentials: 'include'
      });

      if (!response.ok) {
        throw new Error('Failed to fetch wallet analysis from backend');
      }

      const data = await response.json();
      return data;
    } catch (error) {
      console.error('Error fetching wallet analysis:', error);
      return null;
    }
  }

  async getWalletOOFMoments(walletAddress: string) {
    try {
      // Use new Rust backend API for OOF moments
      const response = await fetch(`/v1/moments?wallet=${walletAddress}`, {
        credentials: 'include'
      });

      if (!response.ok) {
        throw new Error('Failed to fetch OOF moments from backend');
      }

      const data = await response.json();
      return data.data || [];
    } catch (error) {
      console.error('Error fetching OOF moments:', error);
      return [];
    }
  }

  async analyzeWallet(walletAddress: string) {
    try {
      // Use new Rust backend API to start wallet analysis
      const response = await apiRequest('POST', '/v1/analyze', {
        wallets: [walletAddress]
      });

      if (!response.ok) {
        throw new Error('Failed to start wallet analysis');
      }

      const data = await response.json();
      return data;
    } catch (error) {
      console.error('Error starting wallet analysis:', error);
      return null;
    }
  }

  async getAnalysisProgress(jobId: string) {
    try {
      // Use new Rust backend API to get analysis progress
      const response = await fetch(`/v1/analyze/${jobId}/stream`, {
        credentials: 'include'
      });

      if (!response.ok) {
        throw new Error('Failed to fetch analysis progress from backend');
      }

      const data = await response.json();
      return data;
    } catch (error) {
      console.error('Error fetching analysis progress:', error);
      return null;
    }
  }
}

export const solanaOnChainService = new SolanaOnChainService();