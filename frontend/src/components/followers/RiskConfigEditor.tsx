import { useState } from "react";
import { clsx } from "clsx";
import type { Follower, RiskConfig } from "@/types";

interface RiskConfigEditorProps {
  follower: Follower;
  onSave: (config: RiskConfig) => void;
  onCancel: () => void;
}

export function RiskConfigEditor({ follower, onSave, onCancel }: RiskConfigEditorProps) {
  const [config, setConfig] = useState<RiskConfig>(
    follower.risk_config || {}
  );

  const update = (key: keyof RiskConfig, value: number | undefined) => {
    setConfig((prev) => ({ ...prev, [key]: value }));
  };

  return (
    <div className="rounded-lg border border-gray-700 bg-gray-950 p-4">
      <h3 className="mb-3 text-sm font-semibold text-gray-200">
        Risk Config — {follower.display_name}
      </h3>
      <div className="grid gap-3">
        <div className="flex items-center justify-between">
          <label className="text-sm text-gray-400">Bankroll ($)</label>
          <input
            type="number"
            className="w-32 rounded border border-gray-700 bg-gray-900 px-2 py-1 text-right text-sm text-gray-200"
            value={config.bankroll ?? ""}
            onChange={(e) => update("bankroll", e.target.value ? Number(e.target.value) : undefined)}
          />
        </div>
        <div className="flex items-center justify-between">
          <label className="text-sm text-gray-400">Copy Scale</label>
          <input
            type="number"
            step="0.01"
            className="w-32 rounded border border-gray-700 bg-gray-900 px-2 py-1 text-right text-sm text-gray-200"
            value={config.copy_scale ?? ""}
            onChange={(e) => update("copy_scale", e.target.value ? Number(e.target.value) : undefined)}
          />
        </div>
        <div className="flex items-center justify-between">
          <label className="text-sm text-gray-400">Min Notional ($)</label>
          <input
            type="number"
            className="w-32 rounded border border-gray-700 bg-gray-900 px-2 py-1 text-right text-sm text-gray-200"
            value={config.min_notional ?? ""}
            onChange={(e) => update("min_notional", e.target.value ? Number(e.target.value) : undefined)}
          />
        </div>
        <div className="flex items-center justify-between">
          <label className="text-sm text-gray-400">Max Notional ($)</label>
          <input
            type="number"
            className="w-32 rounded border border-gray-700 bg-gray-900 px-2 py-1 text-right text-sm text-gray-200"
            value={config.max_notional ?? ""}
            onChange={(e) => update("max_notional", e.target.value ? Number(e.target.value) : undefined)}
          />
        </div>
        <div className="flex items-center justify-between">
          <label className="text-sm text-gray-400">Max Trade Bankroll %</label>
          <input
            type="number"
            step="0.1"
            className="w-32 rounded border border-gray-700 bg-gray-900 px-2 py-1 text-right text-sm text-gray-200"
            value={config.max_trade_bankroll_pct ?? ""}
            onChange={(e) =>
              update("max_trade_bankroll_pct", e.target.value ? Number(e.target.value) : undefined)
            }
          />
        </div>
        <div className="flex items-center justify-between">
          <label className="text-sm text-gray-400">Max Slippage %</label>
          <input
            type="number"
            step="0.1"
            className="w-32 rounded border border-gray-700 bg-gray-900 px-2 py-1 text-right text-sm text-gray-200"
            value={config.max_slippage_pct ?? ""}
            onChange={(e) =>
              update("max_slippage_pct", e.target.value ? Number(e.target.value) : undefined)
            }
          />
        </div>
      </div>
      <div className="mt-4 flex justify-end gap-2">
        <button
          onClick={onCancel}
          className="rounded px-3 py-1 text-sm text-gray-400 hover:text-gray-200"
        >
          Cancel
        </button>
        <button
          onClick={() => onSave(config)}
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
