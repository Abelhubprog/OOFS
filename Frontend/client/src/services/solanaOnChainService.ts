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
      // Use backend API for consistent wallet data
      const response = await fetch(`/api/solana/wallet/${walletAddress}/balance`, {
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
      // Use backend API for token balance
      const response = await fetch(`/api/solana/wallet/${walletAddress}/tokens/${tokenMint}`, {
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
      // Use backend API for token launching
      const response = await apiRequest('POST', '/api/solana/tokens/launch', params);
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
      // Use backend API for token purchases
      const response = await apiRequest('POST', '/api/solana/tokens/buy', {
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
      // Use backend API for token sales
      const response = await apiRequest('POST', '/api/solana/tokens/sell', {
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
      // Use backend API with Jupiter integration
      const response = await apiRequest('POST', '/api/solana/swap', {
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

  async sendSOL(
    fromWallet: string,
    toWallet: string,
    amount: number
  ): Promise<TransactionResult> {
    try {
      // Use backend API for SOL transfers
      const response = await apiRequest('POST', '/api/solana/transfer', {
        fromWallet,
        toWallet,
        amount
      });
      const data = await response.json();

      return {
        success: data.success,
        signature: data.signature,
        error: data.error
      };
    } catch (error) {
      console.error('Error sending SOL:', error);
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  async getRecentTransactions(walletAddress: string): Promise<any[]> {
    try {
      // Use backend API for transaction history
      const response = await fetch(`/api/solana/wallet/${walletAddress}/transactions`, {
        credentials: 'include'
      });

      if (!response.ok) {
        throw new Error('Failed to fetch transactions from backend');
      }

      const data = await response.json();
      return data.transactions || [];
    } catch (error) {
      console.error('Error fetching transactions from backend:', error);

      // Fallback to direct RPC call
      try {
        const publicKey = new PublicKey(walletAddress);
        const signatures = await this.connection.getSignaturesForAddress(
          publicKey,
          { limit: 10 }
        );

        const transactions = await Promise.all(
          signatures.map(async (sig) => {
            const tx = await this.connection.getParsedTransaction(sig.signature);
            return {
              signature: sig.signature,
              blockTime: sig.blockTime,
              slot: sig.slot,
              err: sig.err,
              transaction: tx,
            };
          })
        );

        return transactions.filter(tx => tx.transaction !== null);
      } catch (rpcError) {
        console.error('RPC fallback failed:', rpcError);
        return [];
      }
    }
  }

  async validateTokenMint(mintAddress: string): Promise<boolean> {
    try {
      // Use backend API for token validation
      const response = await fetch(`/api/solana/tokens/${mintAddress}/validate`, {
        credentials: 'include'
      });

      if (!response.ok) {
        throw new Error('Failed to validate token from backend');
      }

      const data = await response.json();
      return data.isValid || false;
    } catch (error) {
      console.error('Error validating token from backend:', error);

      // Fallback to direct RPC call
      try {
        const publicKey = new PublicKey(mintAddress);
        const accountInfo = await this.connection.getAccountInfo(publicKey);
        return accountInfo !== null && accountInfo.owner.equals(TOKEN_PROGRAM_ID);
      } catch (rpcError) {
        console.error('RPC fallback failed:', rpcError);
        return false;
      }
    }
  }

  // Utility method to estimate transaction fees
  async estimateTransactionFee(transaction: Transaction): Promise<number> {
    try {
      // Use backend API for fee estimation
      const response = await apiRequest('POST', '/api/solana/estimate-fee', {
        transaction: transaction.serialize({ requireAllSignatures: false }).toString('base64')
      });
      const data = await response.json();

      return data.estimatedFee || 0.00025;
    } catch (error) {
      console.error('Error estimating transaction fee from backend:', error);

      // Fallback to direct RPC call
      try {
        const { blockhash } = await this.connection.getLatestBlockhash();
        transaction.recentBlockhash = blockhash;

        const fee = await this.connection.getFeeForMessage(
          transaction.compileMessage()
        );

        return fee.value ? fee.value / LAMPORTS_PER_SOL : 0.00025;
      } catch (rpcError) {
        console.error('RPC fallback failed:', rpcError);
        return 0.00025; // Default estimate
      }
    }
  }
}

export const solanaOnChainService = new SolanaOnChainService();
