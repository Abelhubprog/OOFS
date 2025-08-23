import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { useAchievements } from "@/hooks/useAchievements";
import ConfettiBurst, { AchievementConfetti } from "@/components/ConfettiBurst";
import AchievementToast from "@/components/AchievementToast";
import { useOOFTheme } from "@/providers/ThemeProvider";
import { oofDesignSystem } from "@/lib/design-system";
import {
  Home,
  BarChart3,
  Search,
  Star,
  Layers,
  Shield,
  Brain,
  BookOpen,
  Crown,
  Coins,
  Zap,
  Sparkles,
  Clock,
  TrendingUp,
  Wallet,
  Users,
  LogOut,
  Calculator,
  Target,
  Bot,
  ShoppingCart,
  MessageCircle,
  Activity,
  Trophy,
  DollarSign,
  ChevronRight,
  Calendar,
  ArrowUp,
  ArrowDown
} from "lucide-react";
import { Link } from "wouter";
import { useQuery } from "@tanstack/react-query";
import { useOOFAuth } from "@/providers/AuthProvider";

interface GlobalStats {
  totalOOFMoments: number;
  totalUsers: number;
  totalRewards: number;
  recentActivity: string;
}

export default function Dashboard() {
  const { user, isAuthenticated } = useOOFAuth();
  const { emotionalMode, getEmotionalColor, getComponentVariant } = useOOFTheme();
  const {
    unlockAchievement,
    trackPrediction,
    trackTrade,
    activeConfetti,
    clearConfetti,
    recentUnlocks,
    totalPoints
  } = useAchievements();

  const [globalStats, setGlobalStats] = useState<GlobalStats>({
    totalOOFMoments: 1337575,
    totalUsers: 12450,
    totalRewards: 245780,
    recentActivity: "Tracking every lost chance from the depths of missed opportunities"
  });

  const [showToast, setShowToast] = useState(false);
  const [demoConfetti, setDemoConfetti] = useState<string | null>(null);

  const { data: tokens } = useQuery({ queryKey: ['/api/tokens'] });
  const { data: predictions } = useQuery({ queryKey: ['/api/predictions'] });
  const { data: userPredictions } = useQuery({ queryKey: ['/api/predictions/user'] });
  const { data: leaderboard } = useQuery({ queryKey: ['/api/leaderboard'] });

  const { data: userStats } = useQuery({
    queryKey: ['/api/user/stats'],
    queryFn: async () => {
      const response = await fetch('/api/user/stats');
      if (!response.ok) return { oofTokens: 0, totalEarned: 0, rank: 0 };
      return response.json();
    }
  });

  useEffect(() => {
    // Real-time counter updates
    const interval = setInterval(() => {
      setGlobalStats(prev => ({
        ...prev,
        totalOOFMoments: prev.totalOOFMoments + Math.floor(Math.random() * 5)
      }));
    }, 3000);

    return () => clearInterval(interval);
  }, []);

  return (
    <div className="text-white">
      {/* Header with Design System Integration */}
      <div className="text-center mb-8">
        <h1 className="text-6xl font-bold bg-gradient-to-r from-oof-primary-400 to-oof-emotional-primary bg-clip-text text-transparent mb-4">
          OOF
        </h1>
        <h2 className="text-2xl text-oof-primary-200 mb-2">
          The Meme Coin for Missed Opportunities 🔍
        </h2>
        <p className="text-oof-primary-300 max-w-2xl mx-auto">
          Turn your crypto regrets into rewards. Track missed gains, predict the next moon, and earn
          $OOF tokens for your trading insights!
        </p>
      </div>

      {/* Global OOF Moments Counter - Using Design System */}
      <div className="max-w-md mx-auto mb-8">
        <Card className={`${getComponentVariant('card', 'elevated')} border-oof-primary-400 text-white`}>
          <CardContent className="p-6 text-center">
            <h3 className="text-lg font-semibold mb-2">Global OOF Moments</h3>
            <div className="text-4xl font-bold text-oof-emotional-accent mb-2">
              {globalStats.totalOOFMoments.toLocaleString()}
            </div>
            <div className="text-oof-primary-200 text-sm">And counting</div>
            <div className="text-xs text-oof-primary-300 mt-2">
              {globalStats.recentActivity}
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Social Achievement Demo Section - Enhanced with Emotional Modes */}
      <div className="max-w-4xl mx-auto mb-8">
        <Card className={`${getComponentVariant('card', 'interactive')} border-oof-emotional-accent/50 backdrop-blur-lg`}>
          <CardContent className="p-6">
            <div className="text-center mb-6">
              <Trophy className="w-12 h-12 mx-auto mb-3 text-oof-emotional-accent" />
              <h3 className="text-2xl font-bold text-white mb-2">
                🎉 Social Achievement Confetti Burst Demo
              </h3>
              <p className="text-gray-100">
                Experience our celebration system - click buttons to trigger different achievement confetti bursts!
              </p>
            </div>

            <div className="grid md:grid-cols-3 gap-4">
              <Button
                onClick={() => {
                  unlockAchievement('first_login');
                  setDemoConfetti('first_login');
                  setTimeout(() => setDemoConfetti(null), 3000);
                }}
                className={`${getEmotionalColor('healing', 500)} hover:${getEmotionalColor('healing', 600)} text-white p-4 h-auto flex flex-col items-center transition-all duration-300 transform hover:scale-105`}
              >
                <Star className="w-6 h-6 mb-2" />
                <span className="font-semibold">First Login</span>
                <span className="text-xs opacity-80">Welcome celebration</span>
              </Button>

              <Button
                onClick={() => {
                  trackTrade(100);
                  setDemoConfetti('trading');
                  setTimeout(() => setDemoConfetti(null), 3000);
                }}
                className={`bg-oof-emotional-primary hover:bg-oof-emotional-secondary text-white p-4 h-auto flex flex-col items-center transition-all duration-300 transform hover:scale-105`}
              >
                <TrendingUp className="w-6 h-6 mb-2" />
                <span className="font-semibold">Trade Success</span>
                <span className="text-xs opacity-80">Trading milestone</span>
              </Button>

              <Button
                onClick={() => {
                  trackPrediction();
                  setDemoConfetti('prediction');
                  setTimeout(() => setDemoConfetti(null), 3000);
                }}
                className={`${getEmotionalColor('viral', 'gradient')} text-white p-4 h-auto flex flex-col items-center transition-all duration-300 transform hover:scale-105`}
              >
                <Crown className="w-6 h-6 mb-2" />
                <span className="font-semibold">Perfect Prediction</span>
                <span className="text-xs opacity-80">Oracle achievement</span>
              </Button>
            </div>

            {totalPoints > 0 && (
              <div className="mt-4 text-center">
                <Badge className="bg-oof-emotional-accent/20 text-oof-emotional-accent border-oof-emotional-accent/30">
                  Total Achievement Points: {totalPoints}
                </Badge>
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Action Buttons */}
      <div className="flex justify-center space-x-3 mb-8 flex-wrap gap-3">
        <Link href="/campaigns">
          <Button className="bg-gradient-to-r from-green-600 to-blue-600 hover:from-green-700 hover:to-blue-700 text-white px-6 py-3">
            <Target className="w-5 h-5 mr-2" />
            Launch Campaign
          </Button>
        </Link>
        <Link href="/wallet-analyzer">
          <Button className="bg-yellow-600 hover:bg-yellow-700 text-white px-6 py-3">
            <Calculator className="w-5 h-5 mr-2" />
            Calculate My OOFs
          </Button>
        </Link>
        <Link href="/tokens">
          <Button variant="outline" className="border-white/60 text-white hover:bg-white/10 px-6 py-3">
            <Search className="w-5 h-5 mr-2" />
            Analyze Wallet
          </Button>
        </Link>
        <Link href="/staking">
          <Button className="bg-green-600 hover:bg-green-700 text-white px-6 py-3">
            <ShoppingCart className="w-5 h-5 mr-2" />
            Buy $OOF
          </Button>
        </Link>
      </div>

      {/* OOF Campaigns Feature Section */}
      <div className="max-w-4xl mx-auto mb-8">
        <Card className="bg-gradient-to-r from-green-900/50 to-blue-900/50 border-green-400/50 backdrop-blur-lg">
          <CardContent className="p-8">
            <div className="text-center mb-6">
              <div className="w-20 h-20 mx-auto mb-4 bg-gradient-to-r from-green-500 to-blue-500 rounded-full flex items-center justify-center">
                <Target className="w-10 h-10 text-white" />
              </div>
              <h3 className="text-3xl font-bold text-white mb-3">OOF Campaigns</h3>
              <p className="text-gray-100 text-lg">
                Launch viral social media campaigns with crypto rewards
              </p>
            </div>

            <div className="grid md:grid-cols-2 gap-6 mb-6">
              <div className="text-center p-4">
                <Users className="w-8 h-8 text-green-300 mx-auto mb-3" />
                <h4 className="font-semibold text-white mb-2">Community Driven</h4>
                <p className="text-sm text-gray-200">
                  Engage your community across Twitter, TikTok, Farcaster, and Arena
                </p>
              </div>
              <div className="text-center p-4">
                <DollarSign className="w-8 h-8 text-blue-300 mx-auto mb-3" />
                <h4 className="font-semibold text-white mb-2">Crypto Rewards</h4>
                <p className="text-sm text-gray-200">
                  Pay participants in USDC and OOF tokens for authentic engagement
                </p>
              </div>
            </div>

            <div className="text-center">
              <Link href="/campaigns">
                <Button className="bg-gradient-to-r from-green-600 to-blue-600 hover:from-green-700 hover:to-blue-700 text-white px-8 py-3 text-lg">
                  <Target className="w-5 h-5 mr-2" />
                  Create Your First Campaign
                </Button>
              </Link>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Platform Features Grid */}
      <div className="max-w-6xl mx-auto mb-8">
        <h3 className="text-2xl font-bold text-center mb-8 text-white">Explore the OOF Platform</h3>
        <div className="grid md:grid-cols-4 gap-4">
          <Link href="/moments">
            <Card className="bg-purple-900/60 border-purple-400/50 hover:bg-purple-800/70 transition-all duration-300 cursor-pointer">
              <CardContent className="p-6 text-center">
                <Star className="w-10 h-10 text-purple-200 mx-auto mb-3" />
                <h4 className="font-semibold text-white mb-2">OOF Moments</h4>
                <p className="text-sm text-gray-200">Track your missed crypto opportunities</p>
              </CardContent>
            </Card>
          </Link>

          <Link href="/detective">
            <Card className="bg-purple-900/60 border-purple-400/50 hover:bg-purple-800/70 transition-all duration-300 cursor-pointer">
              <CardContent className="p-6 text-center">
                <Shield className="w-10 h-10 text-purple-200 mx-auto mb-3" />
                <h4 className="font-semibold text-white mb-2">OOF Detective</h4>
                <p className="text-sm text-gray-200">AI-powered rug detection and analysis</p>
              </CardContent>
            </Card>
          </Link>

          <Link href="/multiverse">
            <Card className="bg-purple-900/60 border-purple-400/50 hover:bg-purple-800/70 transition-all duration-300 cursor-pointer">
              <CardContent className="p-6 text-center">
                <Layers className="w-10 h-10 text-purple-200 mx-auto mb-3" />
                <h4 className="font-semibold text-white mb-2">OOF Multiverse</h4>
                <p className="text-sm text-gray-200">Explore parallel trading universes</p>
              </CardContent>
            </Card>
          </Link>

          <Link href="/staking">
            <Card className="bg-purple-900/60 border-purple-400/50 hover:bg-purple-800/70 transition-all duration-300 cursor-pointer">
              <CardContent className="p-6 text-center">
                <Coins className="w-10 h-10 text-purple-200 mx-auto mb-3" />
                <h4 className="font-semibold text-white mb-2">OOF Staking</h4>
                <p className="text-sm text-gray-200">Stake OOF tokens and earn passive rewards</p>
              </CardContent>
            </Card>
          </Link>
        </div>
      </div>

      {/* Achievement System Components */}
      {(demoConfetti || activeConfetti) && (
        <ConfettiBurst
          isActive={true}
          intensity="high"
          duration={3000}
          onComplete={() => {
            setDemoConfetti(null);
            clearConfetti();
          }}
        />
      )}

      {recentUnlocks.map((achievement, index) => (
        <AchievementToast
          key={`${achievement.id}-${index}`}
          achievement={achievement}
          isVisible={true}
          onClose={() => {}}
        />
      ))}
    </div>
  );
}
