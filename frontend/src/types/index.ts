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

export type BuyMethod = "percentage" | "fixed_amount" | "exact_copy" | "portfolio_weighted";
export type TradeBehavior = "buys_only" | "buys_sells" | "counter" | "all";
export type StopLossType = "percentage" | "fixed_price" | "skip";

export interface RiskConfig {
  // preferred simple copy config
  copy_mode?: "mirror";
  sizing_mode?: "proportional_bankroll";
  follower_bankroll?: number;
  master_bankroll?: number;

  // sizing
  buy_method?: BuyMethod;
  buy_percentage?: number;
  fixed_amount?: number;
  // execution
  slippage_pct?: number;
  // entry filters
  odds_min?: number; // fraction 0..1
  odds_max?: number; // fraction 0..1
  min_trigger?: number;
  max_trigger?: number | null;
  // behavior
  trade_behavior?: TradeBehavior;
  // exits (enforced in Phase 2)
  stop_loss_type?: StopLossType;
  stop_loss_pct?: number;
  take_profit_pct?: number;
  // spending limits
  max_spend_total?: number;
  max_spend_per_market?: number;
  daily_spend?: number;
  min_buy?: number;
  max_buy?: number | null;
  // market filters (enforced in Phase 2)
  categories?: string[];
  min_liquidity?: number;
  min_volume?: number;
  // misc
  max_days_out?: number;
  mute_notifications?: boolean;
  // legacy / kept for back-compat
  bankroll?: number;
  copy_scale?: number;
  min_notional?: number;
  max_notional?: number;
  max_trade_bankroll_pct?: number;
  max_slippage_pct?: number;
}

export const RECOMMENDED_RISK_CONFIG: RiskConfig = {
  copy_mode: "mirror",
  sizing_mode: "proportional_bankroll",
  follower_bankroll: 1000,
  master_bankroll: 50000,
};
