import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useAuthStore } from "@/stores/authStore";
import { toast } from "@/stores/toastStore";

const API_BASE = "/api/v1";

export class AuthError extends Error {
  constructor() {
    super("Session expired — please log in again");
    this.name = "AuthError";
  }
}

// Guard: prevent multiple simultaneous 401 responses from firing multiple redirects
let handlingAuthError = false;

function handleAuthExpiry() {
  if (handlingAuthError) return;
  handlingAuthError = true;
  useAuthStore.getState().clearToken();
  toast.error("Session expired — redirecting to login");
  setTimeout(() => window.location.replace("/login"), 1200);
}

function getToken(): string | null {
  return useAuthStore.getState().token;
}

async function fetchJson<T>(path: string, options?: RequestInit): Promise<T> {
  const token = getToken();
  const res = await fetch(`${API_BASE}${path}`, {
    headers: {
      "Content-Type": "application/json",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...(options?.headers || {}),
    },
    ...options,
  });
  if (res.status === 401) {
    handleAuthExpiry();
    throw new AuthError();
  }
  if (!res.ok) {
    let msg = `Request failed (${res.status})`;
    try {
      const body = await res.json();
      if (body?.error) msg = body.error;
    } catch {}
    throw new Error(msg);
  }
  return res.json();
}

function onErr(label: string) {
  return (err: unknown) => {
    if (err instanceof AuthError) return; // already handled
    toast.error(`${label}: ${err instanceof Error ? err.message : "Unknown error"}`);
  };
}

// ── Auth ─────────────────────────────────────────────────────────────────────

export function useLogin() {
  return useMutation({
    mutationFn: async (body: { username: string; password: string }) => {
      const res = await fetch(`${API_BASE}/auth/login`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      if (!res.ok) throw new Error("Invalid credentials");
      const data = await res.json();
      useAuthStore.getState().setToken(data.token);
      return data;
    },
    onError: onErr("Login failed"),
  });
}

// ── Queries ───────────────────────────────────────────────────────────────────

export function useMasters() {
  return useQuery({ queryKey: ["masters"], queryFn: () => fetchJson<any[]>("/masters") });
}

export function useFollowers() {
  return useQuery({ queryKey: ["followers"], queryFn: () => fetchJson<any[]>("/followers") });
}

export function useSignalsList() {
  return useQuery({
    queryKey: ["signals"],
    queryFn: () => fetchJson<any[]>("/signals"),
    refetchInterval: 5000,
  });
}

export function useOrders() {
  return useQuery({ queryKey: ["orders"], queryFn: () => fetchJson<any[]>("/orders") });
}

export function usePositions() {
  return useQuery({
    queryKey: ["positions"],
    queryFn: () => fetchJson<any[]>("/positions"),
    refetchInterval: 15000,
  });
}

export function useAlerts() {
  return useQuery({
    queryKey: ["alerts"],
    queryFn: () => fetchJson<any[]>("/alerts"),
    refetchInterval: 30000,
  });
}

export function useRawEvents() {
  return useQuery({ queryKey: ["raw-events"], queryFn: () => fetchJson<any[]>("/raw-events") });
}

export function useMarketMappings() {
  return useQuery({ queryKey: ["market-mappings"], queryFn: () => fetchJson<any[]>("/market-mappings") });
}

export function useSystemStatus() {
  return useQuery({
    queryKey: ["system-status"],
    queryFn: () => fetchJson<any>("/system/status"),
    refetchInterval: 10000,
  });
}

export function useAccountBalances() {
  return useQuery({
    queryKey: ["account-balances"],
    queryFn: () => fetchJson<{
      kalshi_balance: number | null;
      polymarket_balance: number | null;
      kalshi_error: string | null;
      polymarket_error: string | null;
    }>("/accounts/balance"),
    refetchInterval: 30000,
  });
}

export function usePnlSummary() {
  return useQuery({
    queryKey: ["pnl"],
    queryFn: () => fetchJson<Array<{
      follower_account_id: string | null;
      total_invested: number;
      unrealized_pnl: number;
      realized_pnl: number;
      total_pnl: number;
      win_rate: number;
      trade_count: number;
      open_positions: number;
    }>>("/pnl"),
    refetchInterval: 15000,
  });
}

// ── System mutations ──────────────────────────────────────────────────────────

export function useToggleSystem() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => fetchJson<any>("/system/toggle", { method: "POST" }),
    onSuccess: (data) => {
      qc.setQueryData(["system-status"], data);
      qc.invalidateQueries({ queryKey: ["system-status"] });
      toast.success(`Live trading ${data.live_trading_enabled ? "enabled" : "disabled"}`);
    },
    onError: onErr("Failed to toggle trading"),
  });
}

export function useToggleKalshiExecution() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => fetchJson<any>("/system/kalshi-execution/toggle", { method: "POST" }),
    onSuccess: (data) => {
      qc.setQueryData(["system-status"], data);
      qc.invalidateQueries({ queryKey: ["system-status"] });
      toast.success(`Kalshi execution ${data.kalshi_execution_enabled ? "enabled" : "disabled"}`);
    },
    onError: onErr("Failed to toggle Kalshi execution"),
  });
}

export function useResetPaperWallet() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => fetchJson<any>("/system/paper-wallet/reset", { method: "POST" }),
    onSuccess: (data) => {
      qc.setQueryData(["system-status"], data);
      qc.invalidateQueries({ queryKey: ["system-status"] });
      toast.success("Paper wallet reset to $1,000");
    },
    onError: onErr("Failed to reset paper wallet"),
  });
}

// ── Master mutations ──────────────────────────────────────────────────────────

export function useUpdateMasterStatus() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, status }: { id: string; status: string }) =>
      fetchJson(`/masters/${id}/status`, { method: "PATCH", body: JSON.stringify({ status }) }),
    onSuccess: (_, { status }) => {
      qc.invalidateQueries({ queryKey: ["masters"] });
      toast.success(`Master status updated to ${status}`);
    },
    onError: onErr("Failed to update master status"),
  });
}

export function useUpdateMasterWalletAddress() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, wallet_address }: { id: string; wallet_address?: string }) =>
      fetchJson(`/masters/${id}/wallet-address`, { method: "PATCH", body: JSON.stringify({ wallet_address }) }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["masters"] });
      toast.success("Wallet address updated");
    },
    onError: onErr("Failed to update wallet address"),
  });
}

export function useCreateMaster() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: { display_name: string; wallet_address?: string; master_weight?: number }) =>
      fetchJson("/masters", { method: "POST", body: JSON.stringify(body) }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["masters"] });
      toast.success("Master created");
    },
    onError: onErr("Failed to create master"),
  });
}

export function useDeleteMaster() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => fetchJson(`/masters/${id}`, { method: "DELETE" }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["masters"] });
      toast.success("Master deleted");
    },
    onError: onErr("Failed to delete master"),
  });
}

// ── Follower mutations ────────────────────────────────────────────────────────

export function useUpdateFollowerStatus() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, status }: { id: string; status: string }) =>
      fetchJson(`/followers/${id}/status`, { method: "PATCH", body: JSON.stringify({ status }) }),
    onSuccess: (_, { status }) => {
      qc.invalidateQueries({ queryKey: ["followers"] });
      toast.success(`Follower status updated to ${status}`);
    },
    onError: onErr("Failed to update follower status"),
  });
}

export function useUpdateFollowerWalletAddress() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, wallet_address }: { id: string; wallet_address?: string }) =>
      fetchJson(`/followers/${id}/wallet-address`, { method: "PATCH", body: JSON.stringify({ wallet_address }) }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["followers"] });
      toast.success("Wallet address updated");
    },
    onError: onErr("Failed to update wallet address"),
  });
}

export function useUpdateFollowerPlatform() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, platform }: { id: string; platform: string }) =>
      fetchJson(`/followers/${id}/platform`, { method: "PATCH", body: JSON.stringify({ platform }) }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["followers"] });
      toast.success("Platform updated");
    },
    onError: onErr("Failed to update platform"),
  });
}

export function useUpdateFollowerRiskConfig() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, config }: { id: string; config: any }) =>
      fetchJson(`/followers/${id}/risk-config`, { method: "PATCH", body: JSON.stringify(config) }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["followers"] });
      toast.success("Risk config saved");
    },
    onError: onErr("Failed to save risk config"),
  });
}

export function useCreateFollower() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: { display_name: string; wallet_address?: string; platform?: string; risk_config?: any }) =>
      fetchJson("/followers", { method: "POST", body: JSON.stringify(body) }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["followers"] });
      toast.success("Follower created");
    },
    onError: onErr("Failed to create follower"),
  });
}

export function useDeleteFollower() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => fetchJson(`/followers/${id}`, { method: "DELETE" }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["followers"] });
      toast.success("Follower deleted");
    },
    onError: onErr("Failed to delete follower"),
  });
}

// ── Market mapping mutations ───────────────────────────────────────────────────

export function useCreateMarketMapping() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: {
      polymarket_condition_id: string;
      polymarket_slug?: string;
      kalshi_ticker: string;
      kalshi_event_ticker?: string;
      confidence?: string;
      enabled?: boolean;
    }) => fetchJson("/market-mappings", { method: "POST", body: JSON.stringify(body) }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["market-mappings"] });
      toast.success("Market mapping created");
    },
    onError: onErr("Failed to create market mapping"),
  });
}

export function useUpdateMarketMapping() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: {
      id: string;
      polymarket_condition_id: string;
      polymarket_slug?: string;
      kalshi_ticker: string;
      kalshi_event_ticker?: string;
      confidence: string;
      enabled: boolean;
    }) => fetchJson(`/market-mappings/${body.id}`, { method: "PATCH", body: JSON.stringify(body) }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["market-mappings"] });
      toast.success("Market mapping updated");
    },
    onError: onErr("Failed to update market mapping"),
  });
}

export function useUpdateMarketMappingEnabled() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) =>
      fetchJson(`/market-mappings/${id}/enabled`, { method: "PATCH", body: JSON.stringify({ enabled }) }),
    onSuccess: (_, { enabled }) => {
      qc.invalidateQueries({ queryKey: ["market-mappings"] });
      toast.success(`Mapping ${enabled ? "enabled" : "disabled"}`);
    },
    onError: onErr("Failed to toggle mapping"),
  });
}

export function useDeleteMarketMapping() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => fetchJson(`/market-mappings/${id}`, { method: "DELETE" }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["market-mappings"] });
      toast.success("Market mapping deleted");
    },
    onError: onErr("Failed to delete market mapping"),
  });
}

// ── Alert mutations ───────────────────────────────────────────────────────────

export function useAckAlert() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => fetchJson(`/alerts/${id}/ack`, { method: "POST" }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["alerts"] });
      toast.success("Alert acknowledged");
    },
    onError: onErr("Failed to acknowledge alert"),
  });
}

export function useAckAllAlerts() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => fetchJson("/alerts/ack-all", { method: "POST" }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["alerts"] });
    },
    onError: onErr("Failed to acknowledge alerts"),
  });
}
