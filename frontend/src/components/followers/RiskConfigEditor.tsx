import { useMemo, useState } from "react";
import { clsx } from "clsx";
import type { Follower, RiskConfig } from "@/types";
import { RECOMMENDED_RISK_CONFIG } from "@/types";

interface RiskConfigEditorProps {
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

export function RiskConfigEditor({ follower, onSave, onCancel }: RiskConfigEditorProps) {
  const [config, setConfig] = useState<RiskConfig>(() => simpleConfig(follower.risk_config));

  const copyPct = useMemo(() => {
    const followerBankroll = config.follower_bankroll ?? 0;
    const masterBankroll = config.master_bankroll ?? 0;
    if (followerBankroll <= 0 || masterBankroll <= 0) return 0;
    return (followerBankroll / masterBankroll) * 100;
  }, [config.follower_bankroll, config.master_bankroll]);

  const update = (key: "follower_bankroll" | "master_bankroll", value: string) => {
    setConfig((prev) => ({
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
      follower_bankroll: config.follower_bankroll,
      master_bankroll: config.master_bankroll,
    });
  };

  return (
    <div className="rounded-lg border border-gray-700 bg-gray-950 p-4">
      <div className="mb-4 flex items-start justify-between gap-4">
        <div>
          <h3 className="text-sm font-semibold text-gray-200">
            Copy Setup - {follower.display_name}
          </h3>
          <p className="mt-1 text-xs text-gray-500">
            Mirrors master buys and sells. Trade size follows your bankroll ratio.
          </p>
        </div>
        <div className="rounded border border-blue-900/60 bg-blue-950/20 px-3 py-2 text-right">
          <p className="text-[11px] uppercase tracking-wide text-blue-200/70">Copy size</p>
          <p className="text-sm font-semibold text-blue-100">{copyPct.toFixed(2)}%</p>
        </div>
      </div>

      <div className="grid gap-3">
        <label className="flex items-center justify-between gap-3">
          <span className="text-sm text-gray-400">My bankroll ($)</span>
          <input
            type="number"
            min="0"
            step="1"
            className="w-36 rounded border border-gray-700 bg-gray-900 px-2 py-1 text-right text-sm text-gray-200"
            value={config.follower_bankroll ?? ""}
            onChange={(e) => update("follower_bankroll", e.target.value)}
          />
        </label>
        <label className="flex items-center justify-between gap-3">
          <span className="text-sm text-gray-400">Master bankroll ($)</span>
          <input
            type="number"
            min="0"
            step="1"
            className="w-36 rounded border border-gray-700 bg-gray-900 px-2 py-1 text-right text-sm text-gray-200"
            value={config.master_bankroll ?? ""}
            onChange={(e) => update("master_bankroll", e.target.value)}
          />
        </label>
      </div>

      <div className="mt-4 flex justify-end gap-2">
        <button
          onClick={onCancel}
          className="rounded px-3 py-1 text-sm text-gray-400 hover:text-gray-200"
        >
          Cancel
        </button>
        <button
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
  );
}
