// 🌐 Mock Service Worker Setup for API Testing

import { setupServer } from 'msw/node';
import { rest } from 'msw';
import { createMockUser, createMockToken, createMockOOFMoment } from '../setup';

// Mock API handlers
export const handlers = [
  // Authentication endpoints
  rest.get('/api/auth/me', (req, res, ctx) => {
    return res(
      ctx.status(200),
      ctx.json({ user: createMockUser() })
    );
  }),

  rest.post('/api/auth/logout', (req, res, ctx) => {
    return res(
      ctx.status(200),
      ctx.json({ success: true })
    );
  }),

  // User endpoints
  rest.get('/api/user/stats', (req, res, ctx) => {
    return res(
      ctx.status(200),
      ctx.json({
        oofTokens: 1000,
        totalEarned: 5000,
        rank: 42,
        achievements: 15,
        predictions: 25,
        successRate: 68.5
      })
    );
  }),

  // Token endpoints
  rest.get('/api/tokens/trending', (req, res, ctx) => {
    return res(
      ctx.status(200),
      ctx.json({
        tokens: [
          createMockToken({ symbol: 'SOL', name: 'Solana' }),
          createMockToken({ symbol: 'BONK', name: 'Bonk' }),
          createMockToken({ symbol: 'WIF', name: 'Dogwifhat' })
        ]
      })
    );
  }),

  rest.get('/api/tokens/search', (req, res, ctx) => {
    const query = req.url.searchParams.get('q') || '';
    return res(
      ctx.status(200),
      ctx.json({
        tokens: [
          createMockToken({
            symbol: query.toUpperCase(),
            name: `${query} Token`
          })
        ]
      })
    );
  }),

  rest.get('/api/tokens/:mint', (req, res, ctx) => {
    const { mint } = req.params;
    return res(
      ctx.status(200),
      ctx.json({
        token: createMockToken({ mint })
      })
    );
  }),

  rest.get('/api/tokens/live-monitoring', (req, res, ctx) => {
    return res(
      ctx.status(200),
      ctx.json([
        {
          id: 1,
          address: '7GCihgDB8fe6KNjn2MYtkzZcRjQy3t9GHdC8uHYmW2hr',
          name: 'BONK',
          symbol: 'BONK',
          emoji: '🐕',
          deployTime: '2 minutes ago',
          riskScore: 25,
          liquidityUSD: 145000,
          holderCount: 1247,
          whaleActivity: 'High',
          socialBuzz: 78,
          rugRisk: 'Low',
          status: 'safe'
        }
      ])
    );
  }),

  rest.post('/api/tokens/analyze', (req, res, ctx) => {
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        analysis: {
          riskScore: 25,
          riskFactors: ['Low liquidity', 'New token'],
          priceData: {
            current: 0.00001234,
            change24h: 15.6,
            volume24h: 125000
          }
        }
      })
    );
  }),

  rest.post('/api/tokens/community-mark', (req, res, ctx) => {
    return res(
      ctx.status(200),
      ctx.json({ success: true })
    );
  }),

  rest.get('/api/tokens/community-stats', (req, res, ctx) => {
    return res(
      ctx.status(200),
      ctx.json({
        safeMarks: 23,
        warnings: 8,
        dangerReports: 3,
        userStats: {
          totalMarks: 15,
          accuracy: 85
        },
        recentActivity: [
          {
            tokenSymbol: 'BONK',
            action: 'safe',
            userAddress: '9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM',
            timeAgo: '5 minutes ago'
          }
        ]
      })
    );
  }),

  // OOF Moments endpoints
  rest.get('/api/oof-moments/user/:userId', (req, res, ctx) => {
    return res(
      ctx.status(200),
      ctx.json([
        createMockOOFMoment(),
        createMockOOFMoment({ id: 2, title: 'Another OOF Moment' })
      ])
    );
  }),

  rest.post('/api/oof-moments/analyze', (req, res, ctx) => {
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        moments: [
          createMockOOFMoment(),
          createMockOOFMoment({ id: 2, rarity: 'legendary' })
        ]
      })
    );
  }),

  rest.get('/api/oof-moments/analysis-status/:address', (req, res, ctx) => {
    return res(
      ctx.status(200),
      ctx.json({
        allowed: true,
        remaining: 5,
        nextAllowedTime: null
      })
    );
  }),

  // Solana blockchain endpoints
  rest.get('/api/solana/wallet/:address/balance', (req, res, ctx) => {
    return res(
      ctx.status(200),
      ctx.json({ balance: 2.5 })
    );
  }),

  rest.get('/api/solana/wallet/:address/tokens/:mint', (req, res, ctx) => {
    return res(
      ctx.status(200),
      ctx.json({ balance: 1000 })
    );
  }),

  rest.get('/api/solana/wallet/:address/transactions', (req, res, ctx) => {
    return res(
      ctx.status(200),
      ctx.json({
        transactions: [
          {
            signature: 'mock_signature_1',
            blockTime: Date.now() / 1000 - 3600,
            slot: 123456789,
            err: null
          }
        ]
      })
    );
  }),

  rest.post('/api/solana/tokens/launch', (req, res, ctx) => {
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        signature: 'mock_launch_signature',
        tokenMint: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'
      })
    );
  }),

  rest.post('/api/solana/tokens/buy', (req, res, ctx) => {
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        signature: 'mock_buy_signature'
      })
    );
  }),

  rest.post('/api/solana/tokens/sell', (req, res, ctx) => {
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        signature: 'mock_sell_signature'
      })
    );
  }),

  rest.post('/api/solana/swap', (req, res, ctx) => {
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        signature: 'mock_swap_signature'
      })
    );
  }),

  rest.post('/api/solana/transfer', (req, res, ctx) => {
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        signature: 'mock_transfer_signature'
      })
    );
  }),

  // Health check
  rest.get('/health', (req, res, ctx) => {
    return res(
      ctx.status(200),
      ctx.json({
        status: 'healthy',
        timestamp: new Date().toISOString(),
        services: {
          database: 'healthy',
          redis: 'healthy',
          blockchain: 'healthy'
        }
      })
    );
  }),

  // Fallback for unhandled requests
  rest.all('*', (req, res, ctx) => {
    console.warn(`Unhandled ${req.method} request to ${req.url}`);
    return res(
      ctx.status(404),
      ctx.json({ error: 'Endpoint not found' })
    );
  })
];

// Create the server
export const server = setupServer(...handlers);
