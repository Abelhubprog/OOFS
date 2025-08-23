// 🚀 OOF Platform Unified API Client - Production Ready

import { useOOFAuth } from '@/providers/AuthProvider';

// 🎯 API Response Types
interface APIResponse<T = any> {
  data: T;
  success: boolean;
  message?: string;
  errors?: string[];
  metadata?: {
    pagination?: {
      page: number;
      limit: number;
      total: number;
      totalPages: number;
    };
    timestamp: string;
    requestId: string;
  };
}

interface APIError {
  status: number;
  message: string;
  code?: string;
  details?: Record<string, any>;
}

// 🔧 Request Configuration
interface RequestConfig {
  timeout?: number;
  retries?: number;
  cache?: 'no-cache' | '5min' | '1hour' | '1day';
  onProgress?: (progress: number) => void;
  onSuccess?: (data: any) => void;
  requireAuth?: boolean;
  headers?: Record<string, string>;
}

// 📊 OOF Platform Specific Types
interface OOFMomentRequest {
  walletAddress?: string;
  momentType?: 'PAPER_HANDS' | 'DIAMOND_HANDS' | 'RUGPULL_SURVIVOR' | 'WHALE_WATCHER' | 'DUST_COLLECTOR';
  customPrompt?: string;
  isPublic?: boolean;
}

interface OOFMomentResponse {
  id: string;
  title: string;
  description: string;
  narrative: string;
  imageUrl: string;
  rarity: 'legendary' | 'epic' | 'rare' | 'common';
  shareUrl: string;
  generationTime: number;
  cost: number;
  socialText: string;
  hashtags: string[];
  metrics: {
    viralScore: number;
    therapeuticValue: number;
    accuracyRating: number;
  };
}

interface WalletAnalysisRequest {
  address: string;
  includeNFTs?: boolean;
  includeDeFi?: boolean;
  timeframe?: '7d' | '30d' | '90d' | '1y' | 'all';
}

interface WalletAnalysisResponse {
  walletAddress: string;
  totalTransactions: number;
  totalTokensTraded: number;
  biggestGain: {
    token: string;
    symbol: string;
    amount: string;
    profit: string;
    date: string;
  };
  biggestLoss: {
    token: string;
    symbol: string;
    amount: string;
    loss: string;
    date: string;
  };
  oofMoments: OOFMomentResponse[];
  tradingPersonality: {
    riskTolerance: 'conservative' | 'moderate' | 'aggressive' | 'degen';
    fomoSusceptibility: number;
    patienceScore: number;
    paperHandsTendency: number;
  };
  recommendations: string[];
}

interface CampaignRequest {
  name: string;
  description: string;
  targetAudience: 'paper_hands' | 'diamond_hands' | 'degens' | 'all';
  viralMechanisms: string[];
  rewards: {
    type: 'tokens' | 'nft' | 'access';
    amount: string;
  };
  duration: string;
}

interface NFTMintRequest {
  momentId: string;
  network: 'solana' | 'base' | 'ethereum';
  mintPrice?: string;
  royaltyPercentage?: number;
}

// 🏗️ API Client Class
class OOFAPIClient {
  private baseURL: string;
  private defaultTimeout: number = 30000;
  private maxRetries: number = 3;
  private cache = new Map<string, { data: any; timestamp: number; ttl: number }>();

  constructor() {
    this.baseURL = import.meta.env.VITE_API_BASE_URL || '/api';
  }

  // 🔐 Authentication Helper
  private async getAuthHeaders(): Promise<Record<string, string>> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };

    try {
      // Get JWT token from auth provider if available
      const authContext = useOOFAuth();
      if (authContext?.getJWTToken) {
        const token = await authContext.getJWTToken();
        if (token) {
          headers['Authorization'] = `Bearer ${token}`;
        }
      }
    } catch (error) {
      console.warn('Failed to get auth token:', error);
    }

    return headers;
  }

  // 📝 Cache Management
  private getCacheKey(url: string, options?: any): string {
    return `${url}_${JSON.stringify(options || {})}`;
  }

  private getFromCache(key: string): any | null {
    const cached = this.cache.get(key);
    if (!cached) return null;

    const now = Date.now();
    if (now > cached.timestamp + cached.ttl) {
      this.cache.delete(key);
      return null;
    }

    return cached.data;
  }

  private setCache(key: string, data: any, ttlMs: number): void {
    this.cache.set(key, {
      data,
      timestamp: Date.now(),
      ttl: ttlMs
    });
  }

  // 📡 Core Request Method
  private async request<T>(
    method: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH',
    endpoint: string,
    data?: any,
    config: RequestConfig = {}
  ): Promise<T> {
    const {
      timeout = this.defaultTimeout,
      retries = this.maxRetries,
      cache = 'no-cache',
      onProgress,
      onSuccess,
      requireAuth = true,
      headers: customHeaders = {}
    } = config;

    // Check cache for GET requests
    if (method === 'GET' && cache !== 'no-cache') {
      const cacheKey = this.getCacheKey(endpoint, data);
      const cached = this.getFromCache(cacheKey);
      if (cached) {
        return cached;
      }
    }

    // Prepare request
    const url = `${this.baseURL}${endpoint}`;
    const authHeaders = requireAuth ? await this.getAuthHeaders() : {};

    const requestOptions: RequestInit = {
      method,
      headers: {
        ...authHeaders,
        ...customHeaders
      },
      signal: AbortSignal.timeout(timeout)
    };

    if (data && method !== 'GET') {
      requestOptions.body = JSON.stringify(data);
    }

    // Retry logic
    let lastError: any;
    for (let attempt = 0; attempt <= retries; attempt++) {
      try {
        // Progress reporting for uploads
        if (onProgress && method === 'POST') {
          onProgress((attempt / (retries + 1)) * 50); // 50% for request start
        }

        const response = await fetch(url, requestOptions);

        // Progress reporting
        if (onProgress) {
          onProgress(75); // 75% for response received
        }

        // Handle HTTP errors
        if (!response.ok) {
          const errorData = await response.json().catch(() => ({}));
          const apiError: APIError = {
            status: response.status,
            message: errorData.message || response.statusText,
            code: errorData.code,
            details: errorData.details
          };

          // Don't retry certain errors
          if (response.status === 401 || response.status === 403 || response.status === 422) {
            throw apiError;
          }

          throw apiError;
        }

        const result: APIResponse<T> = await response.json();

        // Progress completion
        if (onProgress) {
          onProgress(100);
        }

        // Success callback
        if (onSuccess) {
          onSuccess(result.data);
        }

        // Cache successful GET requests
        if (method === 'GET' && cache !== 'no-cache') {
          const cacheKey = this.getCacheKey(endpoint, data);
          const ttlMs = this.getCacheTTL(cache);
          this.setCache(cacheKey, result.data, ttlMs);
        }

        return result.data;

      } catch (error) {
        lastError = error;

        // Don't retry on auth errors or final attempt
        if (error.status === 401 || error.status === 403 || attempt === retries) {
          break;
        }

        // Exponential backoff
        const delay = Math.pow(2, attempt) * 1000;
        await new Promise(resolve => setTimeout(resolve, delay));
      }
    }

    throw lastError;
  }

  private getCacheTTL(cache: string): number {
    switch (cache) {
      case '5min': return 5 * 60 * 1000;
      case '1hour': return 60 * 60 * 1000;
      case '1day': return 24 * 60 * 60 * 1000;
      default: return 0;
    }
  }

  // 🧠 OOF Moments API
  async generateOOFMoment(request: OOFMomentRequest, onProgress?: (progress: number) => void): Promise<OOFMomentResponse> {
    return this.request('POST', '/moments/generate', request, {
      timeout: 60000, // AI processing takes time
      retries: 1, // Don't retry AI generation
      onProgress
    });
  }

  async getOOFMoments(filters?: {
    walletAddress?: string;
    momentType?: string;
    rarity?: string;
    limit?: number;
    offset?: number;
  }): Promise<OOFMomentResponse[]> {
    const params = new URLSearchParams();
    if (filters) {
      Object.entries(filters).forEach(([key, value]) => {
        if (value !== undefined) params.append(key, value.toString());
      });
    }

    return this.request('GET', `/moments?${params.toString()}`, null, {
      cache: '5min'
    });
  }

  async getOOFMoment(id: string): Promise<OOFMomentResponse> {
    return this.request('GET', `/moments/${id}`, null, {
      cache: '1hour'
    });
  }

  async likeOOFMoment(id: string): Promise<{ likes: number }> {
    return this.request('POST', `/moments/${id}/like`);
  }

  async shareOOFMoment(id: string, platform: 'twitter' | 'farcaster' | 'telegram'): Promise<{ shareUrl: string }> {
    return this.request('POST', `/moments/${id}/share`, { platform });
  }

  // 🔍 Wallet Analysis API
  async analyzeWallet(request: WalletAnalysisRequest, onProgress?: (progress: number) => void): Promise<WalletAnalysisResponse> {
    return this.request('POST', '/wallets/analyze', request, {
      timeout: 45000, // Wallet analysis takes time
      retries: 2,
      onProgress
    });
  }

  async getWalletHistory(address: string): Promise<any[]> {
    return this.request('GET', `/wallets/${address}/history`, null, {
      cache: '5min'
    });
  }

  // 🎯 Campaigns API
  async createCampaign(request: CampaignRequest): Promise<any> {
    return this.request('POST', '/campaigns', request);
  }

  async getCampaigns(filters?: {
    status?: 'active' | 'upcoming' | 'ended';
    limit?: number
  }): Promise<any[]> {
    const params = new URLSearchParams();
    if (filters) {
      Object.entries(filters).forEach(([key, value]) => {
        if (value !== undefined) params.append(key, value.toString());
      });
    }

    return this.request('GET', `/campaigns?${params.toString()}`, null, {
      cache: '5min'
    });
  }

  async joinCampaign(campaignId: string): Promise<any> {
    return this.request('POST', `/campaigns/${campaignId}/join`);
  }

  // 💎 NFT Minting API
  async mintOOFMomentNFT(request: NFTMintRequest, onProgress?: (progress: number) => void): Promise<any> {
    return this.request('POST', '/nft/mint', request, {
      timeout: 120000, // Blockchain operations take time
      retries: 0, // Never retry blockchain operations
      onProgress
    });
  }

  async getNFTMintStatus(txHash: string): Promise<any> {
    return this.request('GET', `/nft/status/${txHash}`, null, {
      cache: '5min'
    });
  }

  // 👤 User Management API
  async getCurrentUser(): Promise<any> {
    return this.request('GET', '/users/me', null, {
      cache: '5min'
    });
  }

  async updateUserProfile(data: any): Promise<any> {
    return this.request('PATCH', '/users/me', data);
  }

  async getUserStats(userId?: string): Promise<any> {
    const endpoint = userId ? `/users/${userId}/stats` : '/users/me/stats';
    return this.request('GET', endpoint, null, {
      cache: '5min'
    });
  }

  // 🏆 Leaderboard API
  async getLeaderboard(type: 'oof_score' | 'moments' | 'viral' = 'oof_score', limit: number = 50): Promise<any[]> {
    return this.request('GET', `/leaderboard/${type}?limit=${limit}`, null, {
      cache: '5min'
    });
  }

  // 📊 Analytics API
  async trackEvent(event: string, properties?: Record<string, any>): Promise<void> {
    try {
      await this.request('POST', '/analytics/track', { event, properties }, {
        requireAuth: false,
        timeout: 5000,
        retries: 0
      });
    } catch (error) {
      // Silently fail analytics to not disrupt user experience
      console.warn('Analytics tracking failed:', error);
    }
  }

  // 🔔 Notifications API
  async getNotifications(): Promise<any[]> {
    return this.request('GET', '/notifications', null, {
      cache: '1min'
    });
  }

  async markNotificationRead(id: string): Promise<void> {
    await this.request('PATCH', `/notifications/${id}`, { read: true });
  }

  // ⚡ Health Check
  async healthCheck(): Promise<{ status: 'healthy' | 'degraded' | 'unhealthy'; services: any }> {
    return this.request('GET', '/health', null, {
      requireAuth: false,
      timeout: 5000,
      retries: 0
    });
  }
}

// 🚀 Create and export singleton instance
export const oofAPI = new OOFAPIClient();

// 🪝 React Hook for API Client
export function useOOFAPI() {
  return {
    api: oofAPI,

    // Helper methods with built-in error handling
    generateMoment: async (request: OOFMomentRequest, onProgress?: (progress: number) => void) => {
      try {
        return await oofAPI.generateOOFMoment(request, onProgress);
      } catch (error) {
        console.error('Failed to generate OOF moment:', error);
        throw error;
      }
    },

    analyzeWallet: async (address: string, onProgress?: (progress: number) => void) => {
      try {
        return await oofAPI.analyzeWallet({ address }, onProgress);
      } catch (error) {
        console.error('Failed to analyze wallet:', error);
        throw error;
      }
    },

    createCampaign: async (request: CampaignRequest) => {
      try {
        return await oofAPI.createCampaign(request);
      } catch (error) {
        console.error('Failed to create campaign:', error);
        throw error;
      }
    },

    mintNFT: async (momentId: string, network: 'solana' | 'base' | 'ethereum' = 'solana', onProgress?: (progress: number) => void) => {
      try {
        return await oofAPI.mintOOFMomentNFT({ momentId, network }, onProgress);
      } catch (error) {
        console.error('Failed to mint NFT:', error);
        throw error;
      }
    }
  };
}

// 🔄 Legacy Compatibility Function (deprecated)
export async function apiRequest(method: string, endpoint: string, data?: any): Promise<Response> {
  console.warn('⚠️ apiRequest is deprecated. Use oofAPI or useOOFAPI hook instead.');

  try {
    const result = await oofAPI.request(method as any, endpoint, data);

    // Return a Response-like object for compatibility
    return {
      ok: true,
      status: 200,
      json: async () => result
    } as Response;
  } catch (error) {
    // Return an error Response-like object
    return {
      ok: false,
      status: error.status || 500,
      json: async () => ({ error: error.message })
    } as Response;
  }
}

export default oofAPI;
