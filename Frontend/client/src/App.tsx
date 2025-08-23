import { Switch, Route } from "wouter";
import { queryClient } from "./lib/queryClient";
import { QueryClientProvider } from "@tanstack/react-query";
import { Toaster } from "@/components/ui/toaster";
import { TooltipProvider } from "@/components/ui/tooltip";
import { useAchievements } from "@/hooks/useAchievements";
import ConfettiBurst, { AchievementConfetti } from "@/components/ConfettiBurst";
import AchievementToast from "@/components/AchievementToast";
import { OOFAuthProvider } from "@/providers/AuthProvider";
import { OOFThemeProvider } from "@/providers/ThemeProvider";
import { useState, useEffect } from "react";

// 🚀 Performance-optimized lazy-loaded components
import {
  LazyLanding,
  LazyDashboard,
  LazyOOFMoments,
  LazyWalletAnalyzer,
  LazyOOFDetectiveAdvanced,
  LazyOOFBattleRoyale,
  LazyOOFStaking,
  LazyOOFSocial,
  LazyOOFCampaigns,
  LazyTokenExplorer,
  LazyTradersArena,
  LazyTimeMachine,
  LazyOOFDetective,
  LazyOOFOrigins,
  LazyOOFLegends,
  LazyOOFRealityBender,
  LazyOOFAirdrop,
  LazyOOFMultiverse,
  LazyProfile,
  LazySlots,
  LazyDocumentation,
  LazyWhitepaper,
  LazyPartnerships,
  LazySupport,
  LazyAPI,
  LazyAchievements,
  usePerformanceMonitoring
} from "@/lib/performance";

import Navigation from "@/components/Navigation";
import Sidebar from "@/components/Sidebar";
import BottomSlider from "@/components/BottomSlider";
import NotFound from "@/pages/not-found";

function Router() {
  const { trackComponentMount } = usePerformanceMonitoring();

  useEffect(() => {
    const cleanup = trackComponentMount('MainRouter');
    return cleanup;
  }, []);

  return (
    <div className="flex min-h-screen bg-gradient-to-br from-purple-900 via-purple-800 to-purple-900">
      <Sidebar />
      <main className="flex-1 lg:ml-64 p-3 sm:p-6 pb-20 overflow-auto">
        <Switch>
          <Route path="/" component={LazyLanding} />
          <Route path="/dashboard" component={LazyDashboard} />
          <Route path="/tokens" component={LazyTradersArena} />
          <Route path="/token-explorer" component={LazyTokenExplorer} />
          <Route path="/oof-moments" component={LazyOOFMoments} />
          <Route path="/multiverse" component={LazyOOFMultiverse} />
          <Route path="/profile" component={LazyProfile} />
          <Route path="/time-machine" component={LazyTimeMachine} />
          <Route path="/wallet-analyzer" component={LazyWalletAnalyzer} />
          <Route path="/slots" component={LazySlots} />
          <Route path="/detective" component={LazyOOFDetective} />
          <Route path="/detective-advanced" component={LazyOOFDetectiveAdvanced} />
          <Route path="/origins" component={LazyOOFOrigins} />
          <Route path="/legends" component={LazyOOFLegends} />
          <Route path="/staking" component={LazyOOFStaking} />
          <Route path="/battle-royale" component={LazyOOFBattleRoyale} />
          <Route path="/reality-bender" component={LazyOOFRealityBender} />
          <Route path="/airdrop" component={LazyOOFAirdrop} />
          <Route path="/social" component={LazyOOFSocial} />
          <Route path="/traders-arena" component={LazyTradersArena} />
          <Route path="/campaigns" component={LazyOOFCampaigns} />

          {/* Secondary pages - lower priority loading */}
          <Route path="/documentation" component={LazyDocumentation} />
          <Route path="/whitepaper" component={LazyWhitepaper} />
          <Route path="/partnerships" component={LazyPartnerships} />
          <Route path="/support" component={LazySupport} />
          <Route path="/api" component={LazyAPI} />
          <Route path="/achievements" component={LazyAchievements} />

          <Route component={NotFound} />
        </Switch>
      </main>
      <BottomSlider />
    </div>
  );
}

function App() {
  return (
    <OOFThemeProvider>
      <OOFAuthProvider>
        <QueryClientProvider client={queryClient}>
          <TooltipProvider>
            <Router />
            <Toaster />
          </TooltipProvider>
        </QueryClientProvider>
      </OOFAuthProvider>
    </OOFThemeProvider>
  );
}

export default App;
