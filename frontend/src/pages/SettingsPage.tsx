import { useSystemStatus, useToggleSystem, useToggleKalshiExecution, useResetPaperWallet } from "@/hooks/useApi";
import { GlobalKillSwitch } from "@/components/layout/GlobalKillSwitch";

export function SettingsPage() {
  const { data: status, isLoading } = useSystemStatus();
  const toggleKalshi = useToggleKalshiExecution();
  const resetWallet = useResetPaperWallet();
  const kalshiEnabled = status?.kalshi_execution_enabled ?? false;
  const paperBalance = status?.paper_wallet_balance ?? 1000;

  return (
    <div className="space-y-4">
      <h2 className="text-xl font-semibold">Settings</h2>
      <div className="rounded-lg border border-gray-800 bg-gray-950 p-4 space-y-6">

        {/* Live Trading Kill Switch */}
        <div className="flex items-center justify-between border-b border-gray-800 pb-4">
          <div>
            <h3 className="text-sm font-medium text-white">Live Trading</h3>
            <p className="text-xs text-gray-500 mt-0.5">
              Enable or disable all live order execution across all followers.
            </p>
          </div>
          <GlobalKillSwitch />
        </div>

        {/* Kalshi Slave Account Toggle */}
        <div className="border-b border-gray-800 pb-4 space-y-3">
          <div className="flex items-start justify-between gap-4">
            <div className="flex-1">
              <h3 className="text-sm font-medium text-white">Kalshi Slave Account</h3>
              <p className="text-xs text-gray-500 mt-0.5">
                <span className="font-medium text-gray-300">Paper Trade</span> — signals are detected
                and logged; orders are simulated against a virtual $1,000 demo wallet in Supabase.
                <br />
                <span className="font-medium text-gray-300">Live Kalshi</span> — mirrors detected
                Polymarket signals as real orders on your Kalshi production account.
              </p>
            </div>
            <div className="flex items-center gap-2 shrink-0">
              <span className={`rounded px-2 py-0.5 text-xs font-medium ${
                kalshiEnabled
                  ? "bg-green-900 text-green-200"
                  : "bg-yellow-900 text-yellow-300"
              }`}>
                {kalshiEnabled ? "LIVE KALSHI" : "PAPER TRADE"}
              </span>
              <button
                onClick={() => toggleKalshi.mutate()}
                disabled={toggleKalshi.isPending}
                className={`rounded px-3 py-1 text-xs font-medium text-white transition-colors ${
                  kalshiEnabled
                    ? "bg-yellow-700 hover:bg-yellow-600"
                    : "bg-emerald-700 hover:bg-emerald-600"
                } disabled:opacity-50 disabled:cursor-not-allowed`}
              >
                {toggleKalshi.isPending ? "Updating..." : kalshiEnabled ? "Switch to Paper" : "Go Live Kalshi"}
              </button>
            </div>
          </div>

          {/* Paper wallet balance + reset */}
          {!kalshiEnabled && (
            <div className="flex items-center justify-between rounded bg-gray-900 border border-gray-800 px-4 py-3">
              <div>
                <p className="text-xs text-gray-500">Demo Wallet Balance (Supabase)</p>
                <p className="text-lg font-semibold text-yellow-300 mt-0.5">
                  ${paperBalance.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                </p>
                <p className="text-xs text-gray-600 mt-0.5">Decreases with each paper trade</p>
              </div>
              <button
                onClick={() => resetWallet.mutate()}
                disabled={resetWallet.isPending}
                className="rounded px-3 py-1.5 text-xs font-medium bg-gray-700 hover:bg-gray-600 text-gray-200 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {resetWallet.isPending ? "Resetting..." : "Reset to $1,000"}
              </button>
            </div>
          )}
        </div>

        {/* System Status Grid */}
        <div>
          <h3 className="text-sm font-medium text-gray-300 mb-3">System Status</h3>
          <div className="grid grid-cols-2 gap-4">
            <div className="rounded bg-gray-900 p-3 border border-gray-800">
              <label className="block text-xs text-gray-500">Worker Mode</label>
              <div className="mt-1 text-sm font-medium text-gray-200">
                {status?.worker_mode || "—"}
              </div>
            </div>
            <div className="rounded bg-gray-900 p-3 border border-gray-800">
              <label className="block text-xs text-gray-500">Live Trading</label>
              <div className="mt-1 text-sm font-medium">
                <span className={status?.live_trading_enabled ? "text-green-400" : "text-red-400"}>
                  {status?.live_trading_enabled ? "Enabled" : "Disabled"}
                </span>
              </div>
            </div>
            <div className="rounded bg-gray-900 p-3 border border-gray-800">
              <label className="block text-xs text-gray-500">Kalshi Mode</label>
              <div className="mt-1 text-sm font-medium">
                <span className={kalshiEnabled ? "text-green-400" : "text-yellow-400"}>
                  {kalshiEnabled ? "Live Orders" : "Paper Trading"}
                </span>
              </div>
              {!kalshiEnabled && (
                <div className="mt-0.5 text-xs text-gray-400">
                  Demo: ${paperBalance.toFixed(2)}
                </div>
              )}
            </div>
            <div className="rounded bg-gray-900 p-3 border border-gray-800">
              <label className="block text-xs text-gray-500">Open Alerts</label>
              <div className="mt-1 text-sm font-medium text-gray-200">
                {isLoading ? "—" : status?.open_alert_count ?? 0}
              </div>
            </div>
            <div className="rounded bg-gray-900 p-3 border border-gray-800">
              <label className="block text-xs text-gray-500">Last Signal</label>
              <div className="mt-1 text-sm font-medium text-gray-200">
                {status?.last_signal_at
                  ? new Date(status.last_signal_at).toLocaleString()
                  : "—"}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
