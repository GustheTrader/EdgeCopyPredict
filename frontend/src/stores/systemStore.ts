import { create } from "zustand";

interface SystemStatus {
  live_trading_enabled: boolean;
  worker_mode: string;
  open_alert_count: number;
  last_signal_at?: string;
}

interface SystemStore {
  status: SystemStatus | null;
  setStatus: (status: SystemStatus) => void;
}

export const useSystemStore = create<SystemStore>((set) => ({
  status: null,
  setStatus: (status) => set({ status }),
}));
