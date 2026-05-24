export interface Master {
  id: string;
  display_name: string;
  status: string;
  master_weight: number;
  wallet_address?: string;
  config?: Record<string, unknown>;
}

export interface Follower {
  id: string;
  display_name: string;
  status: string;
  base_currency: string;
  wallet_address?: string;
  platform: string;
  risk_config?: RiskConfig;
}

export interface RiskConfig {
  bankroll?: number;
  copy_scale?: number;
  min_notional?: number;
  max_notional?: number;
  max_trade_bankroll_pct?: number;
  max_slippage_pct?: number;
}
