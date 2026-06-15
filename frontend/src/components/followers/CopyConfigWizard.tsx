import { useMemo, useState } from "react";
import { clsx } from "clsx";
import type { Follower, RiskConfig } from "@/types";
import { RECOMMENDED_RISK_CONFIG } from "@/types";

interface CopyConfigWizardProps {
  follower: Follower;
  onSave: (config: RiskConfig) => void;
  onCancel: () => void;
}

function simpleConfig(config?: RiskConfig): RiskConfig {
  return {
    ...RECOMMENDED_RISK_CONFIG,
    copy_mode: "mirror",
    sizing_mode: "proportional_bankroll",
    follower_bankroll:
      config?.follower_bankroll ?? config?.bankroll ?? RECOMMENDED_RISK_CONFIG.follower_bankroll,
    master_bankroll: config?.master_bankroll ?? RECOMMENDED_RISK_CONFIG.master_bankroll,
  };
}

export function CopyConfigWizard({ follower, onSave, onCancel }: CopyConfigWizardProps) {
  const [cfg, setCfg] = useState<RiskConfig>(() => simpleConfig(follower.risk_config));

  const copyPct = useMemo(() => {
    const followerBankroll = cfg.follower_bankroll ?? 0;
    const masterBankroll = cfg.master_bankroll ?? 0;
    if (followerBankroll <= 0 || masterBankroll <= 0) return 0;
    return (followerBankroll / masterBankroll) * 100;
  }, [cfg.follower_bankroll, cfg.master_bankroll]);

  const update = (key: "follower_bankroll" | "master_bankroll", value: string) => {
    setCfg((prev) => ({
      ...prev,
      copy_mode: "mirror",
      sizing_mode: "proportional_bankroll",
      [key]: value ? Number(value) : undefined,
    }));
  };

  const save = () => {
    onSave({
      copy_mode: "mirror",
      sizing_mode: "proportional_bankroll",
      follower_bankroll: cfg.follower_bankroll,
      master_bankroll: cfg.master_bankroll,
    });
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
      <div className="w-full max-w-md rounded-lg border border-gray-700 bg-gray-950 p-4 shadow-2xl">
        <div className="mb-4 flex items-start justify-between gap-4">
          <div>
            <h3 className="text-sm font-semibold text-gray-100">
              Copy Setup - {follower.display_name}
            </h3>
            <p className="mt-1 text-xs text-gray-500">
              Mirror the master's trades and scale size by account bankroll.
            </p>
          </div>
          <div className="rounded border border-blue-900/60 bg-blue-950/20 px-3 py-2 text-right">
            <p className="text-[11px] uppercase tracking-wide text-blue-200/70">Copy size</p>
            <p className="text-sm font-semibold text-blue-100">{copyPct.toFixed(2)}%</p>
          </div>
        </div>

        <div className="space-y-3">
          <label className="flex items-center justify-between gap-3 text-sm text-gray-300">
            <span>My bankroll ($)</span>
            <input
              type="number"
              min="0"
              step="1"
              value={cfg.follower_bankroll ?? ""}
              onChange={(e) => update("follower_bankroll", e.target.value)}
              className="w-36 rounded border border-gray-700 bg-gray-900 px-2 py-1 text-right text-sm text-gray-100"
            />
          </label>
          <label className="flex items-center justify-between gap-3 text-sm text-gray-300">
            <span>Master bankroll ($)</span>
            <input
              type="number"
              min="0"
              step="1"
              value={cfg.master_bankroll ?? ""}
              onChange={(e) => update("master_bankroll", e.target.value)}
              className="w-36 rounded border border-gray-700 bg-gray-900 px-2 py-1 text-right text-sm text-gray-100"
            />
          </label>
        </div>

        <div className="mt-4 flex justify-end gap-2">
          <button
            type="button"
            onClick={onCancel}
            className="rounded px-3 py-1 text-sm text-gray-400 hover:text-gray-200"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={save}
            className={clsx(
              "rounded px-3 py-1 text-sm font-medium",
              "bg-blue-600 text-white hover:bg-blue-500"
            )}
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
