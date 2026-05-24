import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useAuthStore } from "@/stores/authStore";
import { AuthError } from "@/hooks/useApi";
import { Toaster } from "@/components/ui/Toaster";
import { Layout } from "@/components/layout/Layout";
import { LoginPage } from "@/pages/LoginPage";
import { Dashboard } from "@/pages/Dashboard";
import { SignalsPage } from "@/pages/SignalsPage";
import { MastersPage } from "@/pages/MastersPage";
import { FollowersPage } from "@/pages/FollowersPage";
import { PositionsPage } from "@/pages/PositionsPage";
import { OrdersPage } from "@/pages/OrdersPage";
import { AlertsPage } from "@/pages/AlertsPage";
import { SettingsPage } from "@/pages/SettingsPage";
import { RawEventsPage } from "@/pages/RawEventsPage";
import { MarketMappingsPage } from "@/pages/MarketMappingsPage";
import { TradeHistoryPage } from "@/pages/TradeHistoryPage";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      // Never retry auth failures — they won't self-heal and cause spammy redirects
      retry: (_, error) => !(error instanceof AuthError),
      staleTime: 0,
    },
    mutations: {
      retry: false,
    },
  },
});

function RequireAuth({ children }: { children: React.ReactNode }) {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  return isAuthenticated ? <>{children}</> : <Navigate to="/login" replace />;
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <Routes>
          <Route path="/login" element={<LoginPage />} />
          <Route
            element={
              <RequireAuth>
                <Layout />
              </RequireAuth>
            }
          >
            <Route path="/" element={<Dashboard />} />
            <Route path="/trade-history" element={<TradeHistoryPage />} />
            <Route path="/signals" element={<SignalsPage />} />
            <Route path="/masters" element={<MastersPage />} />
            <Route path="/followers" element={<FollowersPage />} />
            <Route path="/positions" element={<PositionsPage />} />
            <Route path="/orders" element={<OrdersPage />} />
            <Route path="/alerts" element={<AlertsPage />} />
            <Route path="/settings" element={<SettingsPage />} />
            <Route path="/raw-events" element={<RawEventsPage />} />
            <Route path="/market-mappings" element={<MarketMappingsPage />} />
          </Route>
        </Routes>
      </BrowserRouter>
      <Toaster />
    </QueryClientProvider>
  );
}

export default App;
