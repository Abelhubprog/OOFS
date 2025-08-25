import React, { useState, useEffect, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { useOOFAuth } from '@/providers/AuthProvider';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiRequest } from '@/lib/queryClient';
import { useToast } from '@/hooks/use-toast';
import { useMomentsStream } from '@/hooks/useMomentsStream';

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from '@/components/ui/dialog';
import { Textarea } from '@/components/ui/textarea';
import { Progress } from '@/components/ui/progress';
import {
  Sparkles,
  Brain,
  Palette,
  Network,
  CheckCircle,
  Clock,
  Heart,
  MessageCircle,
  Share2,
  Download,
  Trophy,
  Zap,
  Star,
  ExternalLink,
  Crown,
  Rocket,
  Target,
  DollarSign,
  Loader2,
  Send,
  ArrowUp,
  ArrowDown,
  Eye,
  Users,
  Gem,
  Coins,
  Reply,
  MoreHorizontal,
  Copy,
  X,
  RefreshCw
} from 'lucide-react';

// Types for the OOF Moments system
interface OOFMoment {
  id: string | number;
  title: string;
  description: string;
  quote: string;
  rarity: 'legendary' | 'epic' | 'rare';
  momentType: 'paper_hands' | 'dust_collector' | 'gains_master';
  tokenSymbol: string;
  tokenAddress: string;
  walletAddress: string;
  userId?: string;
  cardMetadata: {
    background: string;
    emoji: string;
    textColor: string;
    accentColor: string;
    gradientFrom: string;
    gradientTo: string;
  };
  socialStats: {
    upvotes: number;
    downvotes: number;
    likes: number;
    comments: number;
    shares: number;
    views: number;
  };
  hashtags: string[];
  zoraAddress?: string;
  isPublic: boolean;
  createdAt: Date;
}

interface Comment {
  id: number;
  momentId: number;
  userId: string;
  username: string;
  content: string;
  replies: Comment[];
  likes: number;
  createdAt: Date;
}

interface GenerationProgress {
  stage: 'analyzing' | 'detecting' | 'designing' | 'posting' | 'complete';
  progress: number;
  message: string;
  agentActive: string;
}

// Revolutionary AI Agent Status Component
const AIAgentStatus: React.FC<{ progress: GenerationProgress }> = ({ progress }) => {
  const agents = [
    {
      id: 'scout',
      name: 'Scout Agent',
      icon: Brain,
      description: 'Scanning wallet history',
      color: 'from-blue-500 to-cyan-500',
      status: 'Analyzing 50,000+ transactions...'
    },
    {
      id: 'director',
      name: 'Director Agent',
      icon: Target,
      description: 'Crafting emotional narratives',
      color: 'from-purple-500 to-pink-500',
      status: 'Generating epic storylines...'
    },
    {
      id: 'artist',
      name: 'Art Agent',
      icon: Palette,
      description: 'Creating stunning visuals',
      color: 'from-orange-500 to-red-500',
      status: 'Designing legendary cards...'
    },
    {
      id: 'publisher',
      name: 'Zora Agent',
      icon: Network,
      description: 'Cross-chain NFT minting',
      color: 'from-green-500 to-emerald-500',
      status: 'Publishing to Base network...'
    }
  ];

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.9 }}
      animate={{ opacity: 1, scale: 1 }}
      className="bg-gradient-to-br from-purple-900/80 to-pink-900/80 backdrop-blur-xl rounded-3xl p-8 border border-purple-700/50 relative overflow-hidden"
    >
      {/* Animated Background */}
      <motion.div
        className="absolute inset-0 bg-gradient-to-r from-purple-600/10 via-pink-600/10 to-cyan-600/10"
        animate={{ scale: [1, 1.05, 1], rotate: [0, 1, 0] }}
        transition={{ duration: 6, repeat: Infinity, ease: "easeInOut" }}
      />

      <div className="relative z-10">
        <motion.div
          className="flex items-center justify-center mb-6"
          initial={{ y: -20, opacity: 0 }}
          animate={{ y: 0, opacity: 1 }}
        >
          <motion.div
            animate={{ rotate: 360 }}
            transition={{ duration: 3, repeat: Infinity, ease: "linear" }}
            className="mr-3"
          >
            <Sparkles className="text-yellow-400 w-8 h-8" />
          </motion.div>
          <h3 className="text-3xl font-bold bg-gradient-to-r from-yellow-300 to-orange-300 bg-clip-text text-transparent">
            Multi-Agent AI Orchestra
          </h3>
          <Badge className="ml-3 bg-gradient-to-r from-green-500 to-emerald-500 text-white border-0">
            Live Processing
          </Badge>
        </motion.div>

        <motion.div
          className="mb-8"
          initial={{ width: 0 }}
          animate={{ width: "100%" }}
        >
          <div className="flex items-center justify-between mb-3">
            <span className="text-lg font-semibold text-purple-200">Overall Progress</span>
            <span className="text-2xl font-bold text-yellow-400">{progress.progress}%</span>
          </div>
          <div className="h-4 bg-purple-900/50 rounded-full overflow-hidden">
            <motion.div
              className="h-full bg-gradient-to-r from-yellow-400 via-orange-400 to-red-400"
              initial={{ width: 0 }}
              animate={{ width: `${progress.progress}%` }}
              transition={{ duration: 0.8, ease: "easeOut" }}
            />
          </div>
        </motion.div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          {agents.map((agent, index) => {
            const Icon = agent.icon;
            const isActive = progress.agentActive === agent.id;
            const isComplete = progress.progress > (index + 1) * 25;

            return (
              <motion.div
                key={agent.id}
                initial={{ opacity: 0, y: 50 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: index * 0.2 }}
                whileHover={{ scale: 1.05, y: -5 }}
                className={`relative p-6 rounded-2xl border transition-all duration-500 overflow-hidden group ${
                  isActive
                    ? 'border-yellow-400 shadow-yellow-400/25 shadow-2xl'
                    : isComplete
                      ? 'border-green-400 shadow-green-400/25 shadow-xl'
                      : 'border-purple-700/50 hover:border-purple-600'
                }`}
              >
                {/* Background Gradient */}
                <motion.div
                  className={`absolute inset-0 bg-gradient-to-r ${agent.color} ${
                    isActive ? 'opacity-20' : isComplete ? 'opacity-10' : 'opacity-0 group-hover:opacity-5'
                  } transition-opacity duration-500`}
                />

                {/* Status Indicator */}
                <div className="absolute top-4 right-4">
                  {isComplete && (
                    <motion.div
                      initial={{ scale: 0 }}
                      animate={{ scale: 1 }}
                      className="w-6 h-6 bg-green-400 rounded-full flex items-center justify-center"
                    >
                      <CheckCircle className="w-4 h-4 text-white" />
                    </motion.div>
                  )}
                  {isActive && (
                    <motion.div
                      animate={{ rotate: 360 }}
                      transition={{ duration: 2, repeat: Infinity, ease: "linear" }}
                      className="w-6 h-6 bg-yellow-400 rounded-full flex items-center justify-center"
                    >
                      <Clock className="w-4 h-4 text-yellow-900" />
                    </motion.div>
                  )}
                </div>

                <div className="relative z-10">
                  <motion.div
                    className={`w-14 h-14 mb-4 bg-gradient-to-r ${agent.color} rounded-2xl flex items-center justify-center`}
                    whileHover={{ rotate: 15 }}
                  >
                    <Icon className="w-7 h-7 text-white" />
                  </motion.div>

                  <h4 className="text-lg font-bold text-white mb-2">{agent.name}</h4>
                  <p className="text-sm text-purple-300 mb-3">{agent.description}</p>

                  {isActive && (
                    <motion.div
                      initial={{ opacity: 0 }}
                      animate={{ opacity: 1 }}
                      className="text-xs text-yellow-300 font-medium"
                    >
                      {agent.status}
                    </motion.div>
                  )}

                  {isComplete && (
                    <motion.div
                      initial={{ opacity: 0 }}
                      animate={{ opacity: 1 }}
                      className="text-xs text-green-300 font-medium"
                    >
                      ✓ Task completed successfully
                    </motion.div>
                  )}
                </div>
              </motion.div>
            );
          })}
        </div>

        <motion.div
          className="mt-8 text-center"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 1 }}
        >
          <div className="text-lg text-purple-200 mb-2">{progress.message}</div>
          <div className="text-sm text-purple-400">
            Transforming your crypto regrets into legendary moments...
          </div>
        </motion.div>
      </div>
    </motion.div>
  );
};

// Enhanced OOF Card Component with multi-chain support and $OOF token utility
const OOFCard: React.FC<{
  moment: OOFMoment & {
    chain?: string;
    analysis?: any;
    imageUrl?: string;
    mintedOnZora?: boolean;
    zoraMintUrl?: string;
  };
  isOwner: boolean;
  onInteraction: (type: string, momentId: number) => void;
}> = ({ moment, isOwner, onInteraction }) => {
  const [showComments, setShowComments] = useState(false);
  const [newComment, setNewComment] = useState('');
  const [userVote, setUserVote] = useState<'up' | 'down' | null>(null);
  const [showZoraOptions, setShowZoraOptions] = useState(false);
  const [oofAmount, setOofAmount] = useState(25);
  const [isPosting, setIsPosting] = useState(false);

  const getRarityGlow = (rarity: string) => {
    switch (rarity) {
      case 'legendary': return 'shadow-2xl shadow-yellow-500/50';
      case 'epic': return 'shadow-xl shadow-purple-500/50';
      case 'rare': return 'shadow-lg shadow-blue-500/50';
      default: return '';
    }
  };

  const handleVote = (voteType: 'up' | 'down') => {
    setUserVote(userVote === voteType ? null : voteType);
    onInteraction(voteType === 'up' ? 'upvote' : 'downvote', moment.id);
  };

  const handleZoraPost = async (_purchaseTokens: boolean = false) => {
    console.warn('Zora posting is disabled until real on-chain flow is implemented.');
  };

  const getCardTypeInfo = (type: string) => {
    switch (type) {
      case 'max_gains': return { icon: Trophy, color: 'text-yellow-400', label: 'Max Gains' };
      case 'dusts': return { icon: Star, color: 'text-gray-400', label: 'Dusts' };
      case 'lost_opportunities': return { icon: Target, color: 'text-red-400', label: 'Lost Opportunity' };
      default: return { icon: Trophy, color: 'text-purple-400', label: 'OOF Moment' };
    }
  };

  const handleDownloadOrZora = () => {
    if (isOwner) {
      if (moment.mintedOnZora) {
        // Already on Zora, open the link
        window.open(moment.zoraMintUrl, '_blank');
      } else {
        // Show Zora posting options
        setShowZoraOptions(true);
      }
    } else {
      // Redirect to Zora token purchase for non-owners
      if (moment.zoraMintUrl) {
        window.open(moment.zoraMintUrl, '_blank');
      }
    }
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className={`bg-gradient-to-br ${moment.cardMetadata.gradientFrom} ${moment.cardMetadata.gradientTo} rounded-xl p-6 text-white ${getRarityGlow(moment.rarity)} border border-white/20`}
    >
      {/* Prefer server-rendered card image when available */}
      {moment.imageUrl ? (
        <div className="mb-4 overflow-hidden rounded-xl border border-white/10 bg-black/20">
          <img
            src={moment.imageUrl}
            alt={moment.title}
            className="w-full h-auto object-cover"
            onError={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }}
          />
        </div>
      ) : null}
      {/* Enhanced Card Header */}
      <div className="flex items-start justify-between mb-4">
        <div className="flex items-center space-x-3">
          <span className="text-4xl">{moment.cardMetadata.emoji}</span>
          <div>
            <h3 className="text-xl font-bold flex items-center">
              {moment.title}
              {moment.rarity === 'legendary' && <Crown className="ml-2 text-yellow-400" size={20} />}
            </h3>
            <p className="text-white/70 text-sm">{moment.description}</p>
            <div className="flex items-center space-x-2 mt-2">
              {moment.chain && (
                <Badge className="bg-blue-600/20 text-blue-300 text-xs">
                  {moment.chain.toUpperCase()}
                </Badge>
              )}
              {(() => {
                const typeInfo = getCardTypeInfo(moment.momentType);
                const TypeIcon = typeInfo.icon;
                return (
                  <Badge className={`bg-black/20 ${typeInfo.color} text-xs flex items-center`}>
                    <TypeIcon className="w-3 h-3 mr-1" />
                    {typeInfo.label}
                  </Badge>
                );
              })()}
              {moment.mintedOnZora && (
                <Badge className="bg-green-600/20 text-green-300 text-xs">
                  <ExternalLink className="w-3 h-3 mr-1" />
                  On Zora
                </Badge>
              )}
            </div>
          </div>
        </div>
        <div className="flex flex-col items-end space-y-2">
          <Badge variant="secondary" className="bg-white/20">
            {moment.rarity}
          </Badge>
          {moment.analysis?.metrics?.oofScore && (
            <Badge className="bg-purple-600/20 text-purple-300 text-xs">
              OOF Score: {moment.analysis.metrics.oofScore}
            </Badge>
          )}
        </div>
      </div>

      {/* AI-Generated Quote */}
      <div className="mb-4 p-4 bg-black/20 rounded-lg border-l-4 border-white/50">
        <p className="text-lg italic">"{moment.quote}"</p>
      </div>

      {/* Hashtags */}
      <div className="flex flex-wrap gap-2 mb-4">
        {moment.hashtags.map((tag) => (
          <span
            key={tag}
            className="bg-white/20 text-white text-xs font-semibold px-2 py-1 rounded-full"
          >
            {tag}
          </span>
        ))}
      </div>

      {/* Unique Social Actions */}
      <div className="flex items-center justify-between pt-4 border-t border-white/20">
        <div className="flex items-center space-x-4">
          {/* Unique Rocket/Target Voting System */}
          <motion.button
            whileHover={{ scale: 1.1 }}
            whileTap={{ scale: 0.9 }}
            onClick={() => handleVote('up')}
            className={`flex items-center space-x-1 transition-colors ${
              userVote === 'up' ? 'text-green-400' : 'text-white/60 hover:text-green-400'
            }`}
          >
            <Rocket size={20} />
            <span>{moment.socialStats.upvotes}</span>
          </motion.button>

          <motion.button
            whileHover={{ scale: 1.1 }}
            whileTap={{ scale: 0.9 }}
            onClick={() => handleVote('down')}
            className={`flex items-center space-x-1 transition-colors ${
              userVote === 'down' ? 'text-red-400' : 'text-white/60 hover:text-red-400'
            }`}
          >
            <Target size={20} />
            <span>{moment.socialStats.downvotes}</span>
          </motion.button>

          <motion.button
            whileHover={{ scale: 1.1 }}
            onClick={() => setShowComments(!showComments)}
            className="flex items-center space-x-1 text-white/60 hover:text-blue-400 transition-colors"
          >
            <MessageCircle size={20} />
            <span>{moment.socialStats.comments}</span>
          </motion.button>

          <motion.button
            whileHover={{ scale: 1.1 }}
            onClick={() => onInteraction('like', moment.id)}
            className="flex items-center space-x-1 text-white/60 hover:text-pink-400 transition-colors"
          >
            <Heart size={20} />
            <span>{moment.socialStats.likes}</span>
          </motion.button>
        </div>

        <div className="flex items-center space-x-3">
          <motion.button
            whileHover={{ scale: 1.1 }}
            onClick={() => onInteraction('share', moment.id)}
            className="text-white/60 hover:text-white transition-colors"
          >
            <Share2 size={20} />
          </motion.button>

          <motion.button
            whileHover={{ scale: 1.1 }}
            onClick={handleDownloadOrZora}
            className={`transition-colors ${
              isOwner
                ? moment.mintedOnZora
                  ? 'text-white/60 hover:text-green-400'
                  : 'text-white/60 hover:text-yellow-400'
                : 'text-white/60 hover:text-yellow-400'
            }`}
            title={
              isOwner
                ? moment.mintedOnZora
                  ? 'View on Zora'
                  : 'Post to Zora'
                : 'Buy on Zora'
            }
          >
            {isOwner
              ? moment.mintedOnZora
                ? <ExternalLink size={20} />
                : <Coins size={20} />
              : <Coins size={20} />
            }
          </motion.button>
        </div>
      </div>

      {/* Zora Posting Options */}
      <AnimatePresence>
        {showZoraOptions && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="mt-4 pt-4 border-t border-white/20"
          >
            <div className="bg-black/20 rounded-lg p-4">
              <h4 className="text-lg font-bold mb-4 text-center">🎨 Post to Zora</h4>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                {/* Free Posting */}
                <motion.button
                  whileHover={{ scale: 1.02 }}
                  whileTap={{ scale: 0.98 }}
                  onClick={() => handleZoraPost(false)}
                  disabled={isPosting}
                  className="p-4 bg-gradient-to-r from-blue-600/20 to-cyan-600/20 rounded-lg border border-blue-500/30 hover:border-blue-400"
                >
                  <div className="text-center">
                    <Sparkles className="w-8 h-8 mx-auto mb-2 text-blue-400" />
                    <div className="font-bold text-white">Free Posting</div>
                    <div className="text-sm text-blue-300 mb-2">Using $ZORA or Base ETH</div>
                    <div className="text-xs text-white/60">No token purchase</div>
                  </div>
                </motion.button>

                {/* $OOF Token Purchase */}
                <motion.button
                  whileHover={{ scale: 1.02 }}
                  whileTap={{ scale: 0.98 }}
                  onClick={() => handleZoraPost(true)}
                  disabled={isPosting}
                  className="p-4 bg-gradient-to-r from-green-600/20 to-emerald-600/20 rounded-lg border border-green-500/30 hover:border-green-400"
                >
                  <div className="text-center">
                    <Coins className="w-8 h-8 mx-auto mb-2 text-green-400" />
                    <div className="font-bold text-white">$OOF Purchase</div>
                    <div className="text-sm text-green-300 mb-2">Buy first token supply</div>
                    <Input
                      type="number"
                      value={oofAmount}
                      onChange={(e) => setOofAmount(Number(e.target.value))}
                      min="1"
                      max="100"
                      className="bg-white/10 border-white/20 text-white text-center text-sm"
                      onClick={(e) => e.stopPropagation()}
                    />
                    <div className="text-xs text-white/60 mt-1">${oofAmount} worth of tokens</div>
                  </div>
                </motion.button>
              </div>

              <div className="flex justify-between items-center mt-4">
                <button
                  onClick={() => setShowZoraOptions(false)}
                  className="text-white/60 hover:text-white text-sm"
                >
                  Cancel
                </button>
                <div className="text-xs text-white/60">
                  {isPosting ? 'Posting to Zora...' : 'Choose posting option'}
                </div>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Comments Section */}
      <AnimatePresence>
        {showComments && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="mt-4 pt-4 border-t border-white/20"
          >
            <div className="flex items-center space-x-2 mb-4">
              <Input
                value={newComment}
                onChange={(e) => setNewComment(e.target.value)}
                placeholder="Share your thoughts..."
                className="flex-1 bg-white/10 border-white/20 text-white placeholder:text-white/50"
              />
              <Button
                onClick={() => {
                  if (newComment.trim()) {
                    onInteraction('comment', moment.id);
                    setNewComment('');
                  }
                }}
                size="sm"
                className="bg-white/20 hover:bg-white/30"
              >
                <Send size={16} />
              </Button>
            </div>

            {/* Comment list would go here */}
            <div className="space-y-3">
              {/* Placeholder for comments */}
              <div className="text-white/60 text-sm text-center py-4">
                Be the first to comment on this OOF Moment!
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
};

// Real-time Community Feed Component
const CommunityFeed: React.FC = () => {
  // Map backend MomentDto to UI OOFMoment
  const mapMoment = (m: any): OOFMoment => {
    const kind = String(m.kind || '').toLowerCase();
    const t = kind.includes('s2e') ? 'gains_master' : kind.includes('bhd') ? 'paper_hands' : 'dust_collector';
    const sev = parseFloat(m.severity_dec || '0');
    const rarityComputed: OOFMoment['rarity'] = sev > 0.8 ? 'legendary' : sev > 0.4 ? 'epic' : 'rare';
    const emoji = m.display?.emoji || (t === 'gains_master' ? '💎' : t === 'paper_hands' ? '📄' : '🧹');
    const grad = m.display?.gradientFrom && m.display?.gradientTo
      ? { from: m.display.gradientFrom, to: m.display.gradientTo }
      : t === 'gains_master'
        ? { from: 'from-green-600/40', to: 'to-emerald-700/40' }
        : t === 'paper_hands'
          ? { from: 'from-rose-600/40', to: 'to-orange-600/40' }
          : { from: 'from-slate-600/40', to: 'to-gray-700/40' };
    const rarity: OOFMoment['rarity'] = (m.display?.rarity as any) || rarityComputed;
    return {
      id: m.id,
      title: `${kind.toUpperCase()} ${m.tokenSymbol ? `• ${m.tokenSymbol}` : m.mint ? `• ${m.mint.slice(0,6)}` : ''}`.trim(),
      description: m.explain_json?.summary || 'OOF Moment',
      quote: '',
      rarity,
      momentType: t,
      tokenSymbol: m.explain_json?.tokenSymbol || '',
      tokenAddress: m.mint || '',
      walletAddress: m.wallet || '',
      userId: undefined,
      cardMetadata: {
        background: '',
        emoji,
        textColor: '#ffffff',
        accentColor: '#ffffff',
        gradientFrom: grad.from,
        gradientTo: grad.to,
      },
      socialStats: { upvotes: 0, downvotes: 0, likes: 0, comments: 0, shares: 0, views: 0 },
      hashtags: [],
      isPublic: true,
      createdAt: new Date(m.t_event || Date.now()),
      zoraAddress: undefined,
      // Hint for OOFCard to render server-generated image first
      imageUrl: `/api/cards/moment/${encodeURIComponent(m.id)}.png`,
    };
  };

  const { data: publicMoments = [], isLoading } = useQuery({
    queryKey: ['/api/moments', { limit: 30 }],
    queryFn: async () => {
      const response = await fetch('/api/moments?limit=30');
      if (!response.ok) throw new Error('Failed to fetch moments');
      const json = await response.json();
      const list = Array.isArray(json?.data) ? json.data : [];
      return list.map(mapMoment);
    },
    refetchInterval: 10000
  });

  const handleInteraction = (type: string, momentId: number | string) => {
    console.log(`${type} interaction on moment ${momentId}`);
    // Handle social interactions
  };

  // Live SSE stream: merge newest moments into feed
  const { events } = useMomentsStream();
  const streamed = React.useMemo(() => {
    const set = new Map<string | number, any>();
    const incoming: any[] = [];
    for (const ev of events) {
      if (ev.event === 'message' && ev.data && ev.data.id) {
        const mapped = mapMoment(ev.data);
        if (!set.has(mapped.id)) {
          set.set(mapped.id, true);
          incoming.push(mapped);
        }
      }
    }
    return incoming;
  }, [events]);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-16">
        <Loader2 className="animate-spin mr-2" />
        <span>Loading community moments...</span>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold">Community OOF Moments</h2>
        <div className="flex items-center space-x-2 text-sm text-purple-300">
          <div className="w-2 h-2 bg-green-400 rounded-full animate-pulse"></div>
          <span>Live Feed</span>
        </div>
      </div>

      {publicMoments.length > 0 ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {[...streamed, ...publicMoments].reduce((acc: OOFMoment[], m: OOFMoment) => {
            if (!acc.find((x) => x.id === m.id)) acc.push(m);
            return acc;
          }, []).slice(0, 60).map((moment: OOFMoment) => (
            <OOFCard
              key={moment.id}
              moment={moment}
              isOwner={false}
              onInteraction={handleInteraction}
            />
          ))}
        </div>
      ) : (
        <div className="text-center py-16 bg-purple-900/30 rounded-xl border border-purple-700">
          <Trophy className="mx-auto text-yellow-400 h-16 w-16 mb-4" />
          <h3 className="text-2xl font-bold mb-2">No OOF Moments Yet</h3>
          <p className="text-purple-300">Be the first to analyze a wallet and create legendary OOF Moments!</p>
        </div>
      )}
    </div>
  );
};

// Main OOF Moments Page Component
export default function OOFMomentsPage() {
  const { user, primaryWallet } = useOOFAuth();
  const { toast } = useToast();
  const queryClient = useQueryClient();

  const [walletAddress, setWalletAddress] = useState('');
  const [activeTab, setActiveTab] = useState('discover');
  const [isGenerating, setIsGenerating] = useState(false);
  const [generationProgress, setGenerationProgress] = useState<GenerationProgress>({
    stage: 'analyzing',
    progress: 0,
    message: 'Initializing AI agents...',
    agentActive: 'scout'
  });
  const [generatedMoments, setGeneratedMoments] = useState<OOFMoment[]>([]);
  const [analysisStatus, setAnalysisStatus] = useState<any>(null);
  const [isLoadingStatus, setIsLoadingStatus] = useState<boolean>(false);
  const [selectedChain, setSelectedChain] = useState<string>('solana');

  // Fetch wallet-specific moments from backend
  const { data: walletMoments = [], isLoading: loadingWalletMoments, refetch: refetchWalletMoments } = useQuery({
    queryKey: ['/api/moments', { wallet: walletAddress }],
    queryFn: async () => {
      if (!walletAddress) return [] as OOFMoment[];
      const response = await fetch(`/api/moments?wallet=${encodeURIComponent(walletAddress)}&limit=50`);
      if (!response.ok) throw new Error('Failed to fetch wallet moments');
      const json = await response.json();
      const list = Array.isArray(json?.data) ? json.data : [];
      return list.map((m: any) => (mapMoment as any)(m));
    },
    enabled: !!walletAddress
  });

  // No local rate-limit status endpoint; backend applies quotas/rate limits

  // Removed OOF token info (no token features active)

  // Analyze via backend + SSE progress
  const analyzeWalletMutation = useMutation({
    mutationFn: async (address: string) => {
      const res = await fetch('/api/analyze', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ wallets: [address] })
      });
      if (!res.ok) throw new Error('Failed to enqueue analysis');
      return res.json();
    },
    onSuccess: (data) => {
      const jobId = data?.jobId;
      if (!jobId) return;
      try {
        const es = new EventSource(`/api/analyze/${jobId}/stream`);
        es.addEventListener('progress', (e: MessageEvent) => {
          setGenerationProgress({
            stage: 'analyzing',
            progress: Math.min(95, generationProgress.progress + 5),
            message: (e.data as string) || 'Processing...',
            agentActive: 'scout'
          });
        });
        es.addEventListener('done', () => {
          setGenerationProgress({ stage: 'complete', progress: 100, message: 'Analysis complete', agentActive: 'publisher' });
          refetchWalletMoments();
          es.close();
        });
        es.addEventListener('error', () => {
          es.close();
        });
      } catch (err) {
        console.error('SSE init failed', err);
      }
    },
    onError: (error) => {
      toast({ title: 'Analysis Failed', description: error instanceof Error ? error.message : 'Unable to analyze wallet.', variant: 'destructive' });
    }
  });

  const handleAnalyze = async () => {
    if (!walletAddress.trim()) {
      toast({
        title: 'Wallet Required',
        description: `Please enter a ${selectedChain} wallet address`,
        variant: 'destructive'
      });
      return;
    }

    analyzeWalletMutation.mutate(walletAddress);
  };

  const handleInteraction = (type: string, momentId: number) => {
    console.log(`${type} interaction on moment ${momentId}`);
    // Handle social interactions
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-purple-900 via-purple-800 to-pink-900 text-white relative overflow-hidden">
      {/* Animated Background */}
      <div className="absolute inset-0 overflow-hidden">
        <motion.div
          animate={{ rotate: 360 }}
          transition={{ duration: 60, repeat: Infinity, ease: "linear" }}
          className="absolute -top-40 -left-40 w-80 h-80 bg-gradient-to-r from-purple-400/5 to-pink-400/5 rounded-full blur-3xl"
        />
        <motion.div
          animate={{ rotate: -360 }}
          transition={{ duration: 45, repeat: Infinity, ease: "linear" }}
          className="absolute top-1/2 -right-40 w-96 h-96 bg-gradient-to-r from-cyan-400/5 to-blue-400/5 rounded-full blur-3xl"
        />
      </div>

      <div className="relative z-10 container mx-auto px-4 py-8">
        {/* Epic Header */}
        <motion.div
          initial={{ opacity: 0, y: -50 }}
          animate={{ opacity: 1, y: 0 }}
          className="text-center mb-12"
        >
          <motion.h1
            className="text-6xl sm:text-7xl font-black mb-6 bg-gradient-to-r from-yellow-300 via-orange-300 to-red-300 bg-clip-text text-transparent"
            animate={{ scale: [1, 1.02, 1] }}
            transition={{ duration: 4, repeat: Infinity, ease: "easeInOut" }}
          >
            OOF Moments
          </motion.h1>
          <motion.p
            className="text-2xl text-purple-200 mb-8 max-w-4xl mx-auto leading-relaxed"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 0.3 }}
          >
            Transform your crypto trading stories into shareable social media moments
          </motion.p>

          <motion.div
            className="flex justify-center space-x-4 mb-8"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.5 }}
          >
            <Badge className="bg-gradient-to-r from-purple-600 to-pink-600 text-white px-4 py-2 text-sm border-0">
              <Brain className="w-4 h-4 mr-2" />
              AI-Powered Analysis
            </Badge>
            <Badge className="bg-gradient-to-r from-cyan-600 to-blue-600 text-white px-4 py-2 text-sm border-0">
              <Network className="w-4 h-4 mr-2" />
              Cross-Chain NFTs
            </Badge>
            <Badge className="bg-gradient-to-r from-green-600 to-emerald-600 text-white px-4 py-2 text-sm border-0">
              <Coins className="w-4 h-4 mr-2" />
              Monetizable Content
            </Badge>
          </motion.div>
        </motion.div>

        {/* Revolutionary Wallet Input Section */}
        <motion.div
          initial={{ opacity: 0, y: 50 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.7 }}
          className="max-w-4xl mx-auto mb-12"
        >
          <Card className="bg-gradient-to-br from-purple-900/80 to-pink-900/80 backdrop-blur-xl border border-purple-700/50 rounded-3xl p-8 relative overflow-hidden">
            <motion.div
              className="absolute inset-0 bg-gradient-to-r from-purple-600/10 via-pink-600/10 to-cyan-600/10"
              animate={{ scale: [1, 1.02, 1] }}
              transition={{ duration: 6, repeat: Infinity, ease: "easeInOut" }}
            />

            <CardHeader className="relative z-10 pb-6">
              <CardTitle className="text-3xl font-bold text-center mb-4 bg-gradient-to-r from-yellow-300 to-orange-300 bg-clip-text text-transparent">
                🎨 AI-Powered OOF Moments Generator
              </CardTitle>
              <p className="text-purple-200 text-center text-lg">
                Transform your crypto trading stories into shareable social media moments and launch them as tokens on Zora
              </p>
            </CardHeader>

            <CardContent className="relative z-10">
              {/* Chain Selection */}
              <div className="flex justify-center mb-6">
                <div className="flex bg-purple-900/30 rounded-xl p-1 border border-purple-700/50">
                  {['solana', 'base', 'avalanche'].map((chain) => (
                    <button
                      key={chain}
                      onClick={() => setSelectedChain(chain)}
                      className={`px-6 py-2 rounded-lg font-semibold text-sm transition-all duration-300 ${
                        selectedChain === chain
                          ? 'bg-gradient-to-r from-purple-600 to-pink-600 text-white shadow-lg'
                          : 'text-purple-300 hover:text-white hover:bg-purple-800/50'
                      }`}
                    >
                      {chain.charAt(0).toUpperCase() + chain.slice(1)}
                    </button>
                  ))}
                </div>
              </div>

              {/* Rate Limiting Status */}
              {rateLimitStatus && (
                <div className="mb-4 p-3 bg-purple-900/40 rounded-lg border border-purple-700/50">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center space-x-2">
                      <Clock className="w-4 h-4 text-purple-300" />
                      <span className="text-sm text-purple-300">
                        {rateLimitStatus.allowed ? 'Analysis Available' : 'Daily Limit Reached'}
                      </span>
                    </div>
                    {!rateLimitStatus.allowed && (
                      <span className="text-xs text-purple-400">
                        Next: {new Date(rateLimitStatus.nextAllowedTime).toLocaleTimeString()}
                      </span>
                    )}
                  </div>
                </div>
              )}

              <div className="flex flex-col lg:flex-row gap-4 mb-6">
                <div className="flex-1">
                  <Input
                    placeholder={`Enter ${selectedChain} wallet address (supports memecoin tracking)`}
                    value={walletAddress}
                    onChange={(e) => setWalletAddress(e.target.value)}
                    className="bg-purple-900/50 border-purple-700 text-white placeholder-purple-400 text-lg py-6 rounded-2xl"
                    disabled={isGenerating}
                  />
                </div>
                <motion.div whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.95 }}>
                  <Button
                    onClick={handleAnalyze}
                    disabled={isGenerating || (rateLimitStatus && !rateLimitStatus.allowed)}
                    size="lg"
                    className={`font-bold px-8 py-6 text-lg rounded-2xl border-0 relative overflow-hidden group ${
                      rateLimitStatus && !rateLimitStatus.allowed
                        ? 'bg-gray-500 text-gray-300 cursor-not-allowed'
                        : 'bg-gradient-to-r from-yellow-500 to-orange-500 hover:from-yellow-600 hover:to-orange-600 text-yellow-900'
                    }`}
                  >
                    {isGenerating ? (
                      <>
                        <Loader2 className="w-6 h-6 mr-3 animate-spin" />
                        Multi-Chain Analysis...
                      </>
                    ) : rateLimitStatus && !rateLimitStatus.allowed ? (
                      <>
                        <Clock className="w-6 h-6 mr-3" />
                        Daily Limit Reached
                      </>
                    ) : (
                      <>
                        <motion.div
                          className="absolute inset-0 bg-gradient-to-r from-yellow-300/20 to-orange-300/20"
                          initial={{ x: "-100%" }}
                          whileHover={{ x: "100%" }}
                          transition={{ duration: 0.6 }}
                        />
                        <Brain className="w-6 h-6 mr-3" />
                        AI Multi-Chain Analysis
                        <Zap className="w-5 h-5 ml-2" />
                      </>
                    )}
                  </Button>
                </motion.div>
              </div>

              {/* Multi-chain Features Info */}
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-4">
                <div className="text-center p-3 bg-purple-900/30 rounded-lg">
                  <Trophy className="w-6 h-6 mx-auto mb-2 text-yellow-400" />
                  <div className="text-sm font-semibold text-white">Max Gains</div>
                  <div className="text-xs text-purple-300">Best performing tokens</div>
                </div>
                <div className="text-center p-3 bg-purple-900/30 rounded-lg">
                  <Star className="w-6 h-6 mx-auto mb-2 text-gray-400" />
                  <div className="text-sm font-semibold text-white">Dusts Collection</div>
                  <div className="text-xs text-purple-300">Worthless token archive</div>
                </div>
                <div className="text-center p-3 bg-purple-900/30 rounded-lg">
                  <Target className="w-6 h-6 mx-auto mb-2 text-red-400" />
                  <div className="text-sm font-semibold text-white">Lost Opportunities</div>
                  <div className="text-xs text-purple-300">Paper hands moments</div>
                </div>
              </div>

              {/* Token utility section removed until on-chain implementation is live */}

              <div className="text-center text-sm text-purple-400">
                AI agents analyze {selectedChain.toUpperCase()} memecoins across Max Gains, Dusts & Lost Opportunities
              </div>
            </CardContent>
          </Card>
        </motion.div>

        {/* AI Generation Progress */}
        <AnimatePresence>
          {isGenerating && (
            <motion.div
              initial={{ opacity: 0, scale: 0.9 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.9 }}
              className="mb-12"
            >
              <AIAgentStatus progress={generationProgress} />
            </motion.div>
          )}
        </AnimatePresence>

        {/* Enhanced Tabs */}
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.9 }}
        >
          <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
            <TabsList className="grid w-full grid-cols-2 bg-purple-900/50 backdrop-blur-xl rounded-2xl p-2 mb-8 border border-purple-700/50">
              <TabsTrigger
                value="discover"
                className="data-[state=active]:bg-gradient-to-r data-[state=active]:from-purple-600 data-[state=active]:to-pink-600 data-[state=active]:text-white rounded-xl py-3 text-lg font-semibold transition-all duration-300"
              >
                <Eye className="w-5 h-5 mr-2" />
                Discover
              </TabsTrigger>
              <TabsTrigger
                value="my_moments"
                className="data-[state=active]:bg-gradient-to-r data-[state=active]:from-cyan-600 data-[state=active]:to-blue-600 data-[state=active]:text-white rounded-xl py-3 text-lg font-semibold transition-all duration-300"
              >
                <Star className="w-5 h-5 mr-2" />
                My Moments
              </TabsTrigger>
            </TabsList>

            <TabsContent value="discover" className="space-y-6">
              <CommunityFeed />
            </TabsContent>

            <TabsContent value="my_moments" className="space-y-6">
              {walletMoments.length > 0 || generatedMoments.length > 0 ? (
                <motion.div
                  className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8"
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                >
                  {[...walletMoments, ...generatedMoments].map((moment, index) => (
                    <motion.div
                      key={moment.id}
                      initial={{ opacity: 0, y: 50 }}
                      animate={{ opacity: 1, y: 0 }}
                      transition={{ delay: index * 0.1 }}
                    >
                      <OOFCard
                        moment={moment}
                        onInteraction={handleInteraction}
                        isOwner={true}
                      />
                    </motion.div>
                  ))}
                </motion.div>
              ) : (
                <motion.div
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  className="text-center py-16"
                >
                  <div className="text-6xl mb-6">✨</div>
                  <h3 className="text-3xl font-bold text-white mb-4">No OOF Moments yet</h3>
                  <p className="text-purple-300 text-lg mb-8">
                    Analyze your wallet to generate your first legendary moments
                  </p>
                  <motion.div whileHover={{ scale: 1.05 }}>
                    <Button
                      onClick={() => document.querySelector('input')?.focus()}
                      className="bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-700 hover:to-pink-700 px-8 py-4 text-lg rounded-2xl"
                    >
                      <Zap className="w-5 h-5 mr-2" />
                      Get Started
                    </Button>
                  </motion.div>
                </motion.div>
              )}
            </TabsContent>
          </Tabs>
        </motion.div>
      </div>
    </div>
  );
}
