import { useState, useEffect } from "react";
import {
  Shield, AlertTriangle, TrendingUp, Eye, Users,
  Clock, Zap, Search, Filter, RefreshCw, Star
} from 'lucide-react';
import { useMutation, useQuery } from "@tanstack/react-query";
import { apiRequest, queryClient } from "@/lib/queryClient";
import { useOOFAuth } from "@/providers/AuthProvider";
import { useToast } from "@/hooks/use-toast";
import { Button } from "@/components/ui/button";

export default function OOFDetective() {
  const { user, isAuthenticated } = useOOFAuth();
  const { toast } = useToast();
  const [filter, setFilter] = useState('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedToken, setSelectedToken] = useState(null);

  // Load live token monitoring data from backend API
  const { data: liveTokens = [], isLoading: tokensLoading, refetch: refetchTokens } = useQuery({
    queryKey: ['/api/tokens/live-monitoring'],
    queryFn: async () => {
      const response = await fetch('/api/tokens/live-monitoring', {
        credentials: 'include'
      });
      if (!response.ok) throw new Error('Failed to fetch live tokens');
      return response.json();
    },
    refetchInterval: 30000, // Refresh every 30 seconds
    enabled: isAuthenticated
  });

  // Token analysis mutation
  const analyzeTokenMutation = useMutation({
    mutationFn: async (tokenAddress: string) => {
      const response = await apiRequest('POST', '/api/tokens/analyze', {
        tokenAddress,
        userId: user?.id
      });
      return response.json();
    },
    onSuccess: (data) => {
      setSelectedToken(data);
      toast({
        title: 'Analysis Complete',
        description: 'Token analysis updated with latest data'
      });
    },
    onError: (error) => {
      toast({
        title: 'Analysis Failed',
        description: error.message,
        variant: 'destructive'
      });
    }
  });

  // Community validation mutations
  const markTokenMutation = useMutation({
    mutationFn: async ({ tokenAddress, status, comment }: {
      tokenAddress: string,
      status: 'safe' | 'warning' | 'danger',
      comment?: string
    }) => {
      const response = await apiRequest('POST', '/api/tokens/community-mark', {
        tokenAddress,
        status,
        comment,
        userId: user?.id
      });
      return response.json();
    },
    onSuccess: () => {
      refetchTokens();
      toast({
        title: 'Thank you!',
        description: 'Your community validation has been recorded'
      });
    }
  });

  const handleTokenAnalysis = (token) => {
    setSelectedToken(token);
    analyzeTokenMutation.mutate(token.address);
  };

  const handleCommunityMark = (status: 'safe' | 'warning' | 'danger') => {
    if (!selectedToken || !user) {
      toast({
        title: 'Authentication Required',
        description: 'Please connect your wallet to mark tokens',
        variant: 'destructive'
      });
      return;
    }

    markTokenMutation.mutate({
      tokenAddress: selectedToken.address,
      status
    });
  };

  const DetectionRadar = () => (
    <div className="bg-white rounded-xl p-6 shadow-lg">
      <div className="flex justify-between items-center mb-6">
        <h2 className="text-xl font-bold flex items-center">
          <Eye className="text-purple-600 mr-2" />
          Live Detection Radar
        </h2>
        <div className="flex items-center space-x-2">
          <div className="flex items-center space-x-2 text-green-600">
            <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
            <span className="text-sm">Live</span>
          </div>
          <Button
            onClick={() => refetchTokens()}
            disabled={tokensLoading}
            size="sm"
            variant="outline"
          >
            <RefreshCw className={`w-4 h-4 ${tokensLoading ? 'animate-spin' : ''}`} />
          </Button>
        </div>
      </div>

      {tokensLoading ? (
        <div className="space-y-4">
          {[1, 2, 3].map(i => (
            <div key={i} className="animate-pulse">
              <div className="bg-gray-200 rounded-lg p-4 h-32"></div>
            </div>
          ))}
        </div>
      ) : liveTokens.length > 0 ? (
        <div className="space-y-4">
          {liveTokens
            .filter(token =>
              filter === 'all' ||
              token.status === filter ||
              token.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
              token.symbol.toLowerCase().includes(searchQuery.toLowerCase())
            )
            .map(token => (
              <TokenDetectionCard key={token.id} token={token} onSelect={handleTokenAnalysis} />
            ))}
        </div>
      ) : (
        <div className="text-center py-8 text-gray-500">
          <Eye className="w-12 h-12 mx-auto mb-4 text-gray-300" />
          <p>No tokens detected yet. The radar is scanning...</p>
        </div>
      )}
    </div>
  );

  const TokenDetectionCard = ({ token, onSelect }) => (
    <div
      onClick={() => onSelect(token)}
      className={`p-4 rounded-lg border-2 cursor-pointer transition-all hover:shadow-md ${
        token.status === 'danger' ? 'border-red-200 bg-red-50' :
        token.status === 'warning' ? 'border-yellow-200 bg-yellow-50' :
        'border-green-200 bg-green-50'
      }`}
    >
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center space-x-3">
          <div className="text-2xl">{token.emoji || '🪙'}</div>
          <div>
            <div className="font-bold text-lg">{token.name}</div>
            <div className="text-sm text-gray-600">{token.symbol}</div>
          </div>
        </div>
        <div className="text-right">
          <div className={`px-3 py-1 rounded-full text-sm font-bold ${
            (token.riskScore || 0) < 30 ? 'bg-green-100 text-green-700' :
            (token.riskScore || 0) < 70 ? 'bg-yellow-100 text-yellow-700' :
            'bg-red-100 text-red-700'
          }`}>
            Risk: {token.riskScore || 'N/A'}
          </div>
          <div className="text-xs text-gray-500 mt-1">
            {token.deployTime || new Date(token.createdAt).toLocaleString()}
          </div>
        </div>
      </div>

      <div className="grid grid-cols-4 gap-3 text-sm">
        <div className="bg-white p-2 rounded">
          <div className="text-gray-600">Liquidity</div>
          <div className="font-bold">
            ${(token.liquidityUSD || 0).toLocaleString()}
          </div>
        </div>
        <div className="bg-white p-2 rounded">
          <div className="text-gray-600">Holders</div>
          <div className="font-bold">{token.holderCount || 0}</div>
        </div>
        <div className="bg-white p-2 rounded">
          <div className="text-gray-600">Whale Activity</div>
          <div className="font-bold">{token.whaleActivity || 'Low'}</div>
        </div>
        <div className="bg-white p-2 rounded">
          <div className="text-gray-600">Social</div>
          <div className="font-bold">{token.socialBuzz || 0}/100</div>
        </div>
      </div>

      <div className="mt-3 flex items-center justify-between">
        <div className={`text-sm font-bold ${
          token.rugRisk === 'Low' ? 'text-green-600' :
          token.rugRisk === 'Medium' ? 'text-yellow-600' :
          token.rugRisk === 'High' ? 'text-red-600' :
          'text-gray-600'
        }`}>
          Rug Risk: {token.rugRisk || 'Unknown'}
        </div>
        <div className="flex items-center space-x-2">
          <button
            className="text-purple-600 hover:text-purple-700"
            onClick={(e) => {
              e.stopPropagation();
              handleCommunityMark('safe');
            }}
          >
            <Star size={16} />
          </button>
          <button
            className="text-blue-600 hover:text-blue-700"
            onClick={(e) => {
              e.stopPropagation();
              onSelect(token);
            }}
          >
            <TrendingUp size={16} />
          </button>
        </div>
      </div>
    </div>
  );

  const TokenAnalysisPanel = () => (
    <div className="bg-white rounded-xl p-6 shadow-lg">
      <h2 className="text-xl font-bold mb-6">Deep Analysis</h2>

      {analyzeTokenMutation.isPending ? (
        <div className="text-center py-12">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-purple-600 mx-auto mb-4"></div>
          <p className="text-gray-500">Analyzing token...</p>
        </div>
      ) : selectedToken ? (
        <div className="space-y-6">
          <div className="text-center p-6 bg-purple-50 rounded-lg">
            <div className="text-4xl mb-2">{selectedToken.emoji || '🪙'}</div>
            <div className="font-bold text-xl">{selectedToken.name}</div>
            <div className="text-purple-600">{selectedToken.symbol}</div>
          </div>

          <div className="space-y-4">
            <div className="flex justify-between items-center p-3 bg-gray-50 rounded-lg">
              <span className="text-gray-600">Contract Address</span>
              <span className="font-mono text-sm">
                {selectedToken.address?.slice(0, 8)}...{selectedToken.address?.slice(-8)}
              </span>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="p-3 bg-gray-50 rounded-lg">
                <div className="text-sm text-gray-600">Liquidity Pool</div>
                <div className="font-bold">
                  ${(selectedToken.liquidityUSD || 0).toLocaleString()}
                </div>
              </div>
              <div className="p-3 bg-gray-50 rounded-lg">
                <div className="text-sm text-gray-600">Holder Count</div>
                <div className="font-bold">{selectedToken.holderCount || 0}</div>
              </div>
            </div>

            {/* Real-time price data if available */}
            {selectedToken.priceData && (
              <div className="grid grid-cols-3 gap-4">
                <div className="p-3 bg-gray-50 rounded-lg">
                  <div className="text-sm text-gray-600">Current Price</div>
                  <div className="font-bold">${selectedToken.priceData.current}</div>
                </div>
                <div className="p-3 bg-gray-50 rounded-lg">
                  <div className="text-sm text-gray-600">24h Change</div>
                  <div className={`font-bold ${
                    selectedToken.priceData.change24h >= 0 ? 'text-green-600' : 'text-red-600'
                  }`}>
                    {selectedToken.priceData.change24h >= 0 ? '+' : ''}
                    {selectedToken.priceData.change24h}%
                  </div>
                </div>
                <div className="p-3 bg-gray-50 rounded-lg">
                  <div className="text-sm text-gray-600">Volume 24h</div>
                  <div className="font-bold">${selectedToken.priceData.volume24h}</div>
                </div>
              </div>
            )}

            <div className="p-4 border rounded-lg">
              <div className="text-sm font-bold mb-2">AI Risk Assessment</div>
              <div className="w-full bg-gray-200 rounded-full h-2">
                <div
                  className={`h-2 rounded-full ${
                    (selectedToken.riskScore || 0) < 30 ? 'bg-green-500' :
                    (selectedToken.riskScore || 0) < 70 ? 'bg-yellow-500' :
                    'bg-red-500'
                  }`}
                  style={{ width: `${selectedToken.riskScore || 0}%` }}
                />
              </div>
              <div className="text-sm text-gray-600 mt-2">
                {(selectedToken.riskScore || 0) < 30 ? 'Safe to trade' :
                 (selectedToken.riskScore || 0) < 70 ? 'Proceed with caution' :
                 'High risk - avoid trading'}
              </div>
              {selectedToken.riskFactors && (
                <div className="mt-3">
                  <div className="text-xs font-semibold text-gray-700 mb-1">Risk Factors:</div>
                  <div className="text-xs text-gray-600">
                    {selectedToken.riskFactors.join(', ')}
                  </div>
                </div>
              )}
            </div>

            <div className="space-y-2">
              <Button
                className="w-full bg-green-600 hover:bg-green-700"
                onClick={() => handleCommunityMark('safe')}
                disabled={markTokenMutation.isPending}
              >
                {markTokenMutation.isPending ? 'Marking...' : 'Mark as Safe'}
              </Button>
              <Button
                variant="outline"
                className="w-full"
                onClick={() => {
                  // Add to watchlist functionality
                  toast({
                    title: 'Added to Watchlist',
                    description: `${selectedToken.symbol} is now being watched`
                  });
                }}
              >
                Add to Watchlist
              </Button>
              <Button
                variant="outline"
                className="w-full text-red-600 border-red-200 hover:bg-red-50"
                onClick={() => handleCommunityMark('danger')}
                disabled={markTokenMutation.isPending}
              >
                {markTokenMutation.isPending ? 'Reporting...' : 'Report Suspicious'}
              </Button>
            </div>
          </div>
        </div>
      ) : (
        <div className="text-center text-gray-500 py-12">
          <Shield className="w-12 h-12 mx-auto mb-4 text-gray-300" />
          <p>Select a token for detailed analysis</p>
          <p className="text-sm mt-2">Our AI will analyze smart contract patterns, liquidity, and community signals</p>
        </div>
      )}
    </div>
  );

  const CommunityValidation = () => {
    // Load community validation stats from backend
    const { data: communityStats } = useQuery({
      queryKey: ['/api/tokens/community-stats'],
      queryFn: async () => {
        const response = await fetch('/api/tokens/community-stats', {
          credentials: 'include'
        });
        if (!response.ok) throw new Error('Failed to fetch community stats');
        return response.json();
      },
      refetchInterval: 60000 // Refresh every minute
    });

    return (
      <div className="bg-white rounded-xl p-6 shadow-lg">
        <h2 className="text-xl font-bold mb-6 flex items-center">
          <Users className="text-purple-600 mr-2" />
          Community Validation
        </h2>

        <div className="space-y-4">
          <div className="p-4 border rounded-lg">
            <div className="flex items-center justify-between mb-2">
              <div className="font-bold">Recent Reports</div>
              <div className="text-sm text-gray-600">Last 24h</div>
            </div>
            <div className="grid grid-cols-3 gap-4 text-center">
              <div>
                <div className="text-2xl font-bold text-green-600">
                  {communityStats?.safeMarks || 0}
                </div>
                <div className="text-sm text-gray-600">Safe Marks</div>
              </div>
              <div>
                <div className="text-2xl font-bold text-yellow-600">
                  {communityStats?.warnings || 0}
                </div>
                <div className="text-sm text-gray-600">Warnings</div>
              </div>
              <div>
                <div className="text-2xl font-bold text-red-600">
                  {communityStats?.dangerReports || 0}
                </div>
                <div className="text-sm text-gray-600">Danger Reports</div>
              </div>
            </div>
          </div>

          {/* User contribution stats */}
          {user && (
            <div className="p-4 bg-purple-50 rounded-lg">
              <div className="flex items-center justify-between mb-2">
                <div className="font-bold">Your Contributions</div>
                <div className="text-2xl font-bold">{user?.oofTokens || 0}</div>
              </div>
              <div className="text-sm text-purple-600 mb-3">
                Earn $OOF tokens for accurate token validations
              </div>
              <div className="grid grid-cols-2 gap-2 text-sm">
                <div className="text-center p-2 bg-white rounded">
                  <div className="font-bold">{communityStats?.userStats?.totalMarks || 0}</div>
                  <div className="text-gray-600">Total Marks</div>
                </div>
                <div className="text-center p-2 bg-white rounded">
                  <div className="font-bold">{communityStats?.userStats?.accuracy || 0}%</div>
                  <div className="text-gray-600">Accuracy</div>
                </div>
              </div>
            </div>
          )}

          {/* Recent community activities */}
          {communityStats?.recentActivity && (
            <div className="space-y-2">
              <div className="font-bold text-sm">Recent Community Activity</div>
              {communityStats.recentActivity.slice(0, 3).map((activity, index) => (
                <div key={index} className="p-2 bg-gray-50 rounded text-sm">
                  <div className="flex items-center justify-between">
                    <span className="font-mono text-xs">
                      {activity.tokenSymbol}
                    </span>
                    <span className={`text-xs px-2 py-1 rounded ${
                      activity.action === 'safe' ? 'bg-green-100 text-green-700' :
                      activity.action === 'warning' ? 'bg-yellow-100 text-yellow-700' :
                      'bg-red-100 text-red-700'
                    }`}>
                      {activity.action}
                    </span>
                  </div>
                  <div className="text-gray-600 text-xs mt-1">
                    by {activity.userAddress?.slice(0, 6)}... • {activity.timeAgo}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    );
  };

  if (!isAuthenticated) {
    return (
      <div className="max-w-4xl mx-auto p-6">
        <div className="text-center">
          <h1 className="text-3xl font-bold text-purple-900 mb-4">
            OOF Detective
          </h1>
          <p className="text-purple-600 mb-8">
            Please log in to access the detection radar and protect the community.
          </p>
          <a
            href="/api/login"
            className="bg-purple-600 text-white px-6 py-3 rounded-lg hover:bg-purple-700 transition-colors"
          >
            Login to Continue
          </a>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-6xl mx-auto p-6">
      {/* Header */}
      <div className="text-center mb-12">
        <h1 className="text-4xl font-bold text-purple-900 mb-2">
          OOF Detective
        </h1>
        <p className="text-purple-600">
          Real-time monitoring of new memecoin deployments and whale movements
        </p>
      </div>

      {/* Stats Overview */}
      <div className="grid grid-cols-4 gap-6 mb-8">
        <div className="bg-white rounded-xl p-4 shadow-lg">
          <div className="flex items-center space-x-2 text-purple-600 mb-2">
            <Shield size={20} />
            <span>Tokens Scanned</span>
          </div>
          <div className="text-2xl font-bold">1,247</div>
          <div className="text-sm text-green-600">+23 today</div>
        </div>

        <div className="bg-white rounded-xl p-4 shadow-lg">
          <div className="flex items-center space-x-2 text-purple-600 mb-2">
            <AlertTriangle size={20} />
            <span>Rugs Detected</span>
          </div>
          <div className="text-2xl font-bold">43</div>
          <div className="text-sm text-red-600">+2 today</div>
        </div>

        <div className="bg-white rounded-xl p-4 shadow-lg">
          <div className="flex items-center space-x-2 text-purple-600 mb-2">
            <Users size={20} />
            <span>Community Score</span>
          </div>
          <div className="text-2xl font-bold">892</div>
          <div className="text-sm text-green-600">Top 5%</div>
        </div>

        <div className="bg-white rounded-xl p-4 shadow-lg">
          <div className="flex items-center space-x-2 text-purple-600 mb-2">
            <Star size={20} />
            <span>Earned $OOF</span>
          </div>
          <div className="text-2xl font-bold">{user?.oofTokens || 0}</div>
          <div className="text-sm text-purple-600">from detections</div>
        </div>
      </div>

      {/* Search and Filters */}
      <div className="flex items-center space-x-4 bg-white p-4 rounded-xl shadow mb-8">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-purple-400" size={20} />
          <input
            type="text"
            placeholder="Search by token name, symbol, or contract address..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full pl-10 p-2 border rounded-lg focus:ring-2 focus:ring-purple-500 outline-none"
          />
        </div>
        <select
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          className="px-4 py-2 border rounded-lg focus:ring-2 focus:ring-purple-500"
        >
          <option value="all">All Tokens</option>
          <option value="safe">Safe</option>
          <option value="warning">Warning</option>
          <option value="danger">High Risk</option>
        </select>
        <button className="flex items-center space-x-2 px-4 py-2 bg-purple-600 text-white rounded-lg hover:bg-purple-700">
          <RefreshCw size={20} />
          <span>Refresh</span>
        </button>
      </div>

      {/* Main Content */}
      <div className="grid grid-cols-3 gap-8">
        {/* Detection Radar */}
        <div className="col-span-2">
          <DetectionRadar />
        </div>

        {/* Right Column */}
        <div className="space-y-6">
          <TokenAnalysisPanel />
          <CommunityValidation />
        </div>
      </div>
    </div>
  );
}
