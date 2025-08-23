export interface TokenInfo {
  mint: string;
  name: string;
  symbol: string;
  description?: string;
  imageUrl?: string;
  price: number;
  marketCap: number;
  liquidity: number;
  bondingCurveProgress: number;
  createdAt: Date;
  websiteUrl?: string;
  twitterUrl?: string;
  telegramUrl?: string;
  volume24h: number;
  priceChange24h: number;
}

class SolanaTokenService {
  private baseUrl = '/api/solana'; // Use backend API instead of external API

  async getTrendingTokens(): Promise<TokenInfo[]> {
    try {
      const response = await fetch(`${this.baseUrl}/tokens/trending`, {
        credentials: 'include'
      });

      if (!response.ok) {
        throw new Error('Failed to fetch trending tokens from backend');
      }

      const data = await response.json();
      return data.tokens || [];
    } catch (error) {
      console.error('Error fetching trending tokens:', error);
      return [];
    }
  }

  async searchTokens(query: string): Promise<TokenInfo[]> {
    try {
      const response = await fetch(`${this.baseUrl}/tokens/search?q=${encodeURIComponent(query)}`, {
        credentials: 'include'
      });

      if (!response.ok) {
        throw new Error('Failed to search tokens from backend');
      }

      const data = await response.json();
      return data.tokens || [];
    } catch (error) {
      console.error('Error searching tokens:', error);
      return [];
    }
  }

  async getTokenByMint(mint: string): Promise<TokenInfo | null> {
    try {
      const response = await fetch(`${this.baseUrl}/tokens/${mint}`, {
        credentials: 'include'
      });

      if (!response.ok) {
        if (response.status === 404) {
          return null;
        }
        throw new Error('Failed to fetch token from backend');
      }

      const data = await response.json();
      return data.token || null;
    } catch (error) {
      console.error('Error fetching token by mint:', error);
      return null;
    }
  }

  async getTokenPrice(mint: string): Promise<number> {
    try {
      const response = await fetch(`${this.baseUrl}/tokens/${mint}/price`, {
        credentials: 'include'
      });

      if (!response.ok) {
        throw new Error('Failed to fetch token price from backend');
      }

      const data = await response.json();
      return data.price || 0;
    } catch (error) {
      console.error('Error fetching token price:', error);
      return 0;
    }
  }

  async getTokenMetrics(mint: string): Promise<{
    holders: number;
    transactions24h: number;
    liquidity: number;
    marketCap: number;
  }> {
    try {
      const response = await fetch(`${this.baseUrl}/tokens/${mint}/metrics`, {
        credentials: 'include'
      });

      if (!response.ok) {
        throw new Error('Failed to fetch token metrics from backend');
      }

      const data = await response.json();
      return data.metrics || {
        holders: 0,
        transactions24h: 0,
        liquidity: 0,
        marketCap: 0
      };
    } catch (error) {
      console.error('Error fetching token metrics:', error);
      return {
        holders: 0,
        transactions24h: 0,
        liquidity: 0,
        marketCap: 0
      };
    }
  }

  async getTokenHolders(mint: string, limit: number = 100): Promise<{
    address: string;
    balance: number;
    percentage: number;
  }[]> {
    try {
      const response = await fetch(`${this.baseUrl}/tokens/${mint}/holders?limit=${limit}`, {
        credentials: 'include'
      });

      if (!response.ok) {
        throw new Error('Failed to fetch token holders from backend');
      }

      const data = await response.json();
      return data.holders || [];
    } catch (error) {
      console.error('Error fetching token holders:', error);
      return [];
    }
  }
}

export const solanaTokenService = new SolanaTokenService();
