import {
  useSystemStatus,
  useToggleKalshiExecution,
  useResetPaperWallet,
  usePaperWalletMovements,
  usePnlSummary,
  type PaperWalletMovement,
} from "@/hooks/useApi";
import { GlobalKillSwitch } from "@/components/layout/GlobalKillSwitch";

const PAPER_FOLLOWER_ID = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
const WALLET_HISTORY_LIMIT = 1000;
const VISIBLE_LEDGER_ROWS = 50;

const money = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
});

function formatSignedMoney(value: number) {
  return `${value >= 0 ? "+" : ""}${money.format(value)}`;
}

function movementLabel(type: string) {
  return type
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function movementSource(movement: PaperWalletMovement) {
  if (movement.source_type && movement.source_id) return `${movement.source_type}:${movement.source_id.slice(0, 8)}`;
  return movement.source_type || "-";
}

export function SettingsPage() {
  const { data: status, isLoading } = useSystemStatus();
  const toggleKalshi = useToggleKalshiExecution();
  const resetWallet = useResetPaperWallet();
  const { data: pnlSummaries = [] } = usePnlSummary();
  const { data: walletMovements = [] } = usePaperWalletMovements(WALLET_HISTORY_LIMIT);
  const kalshiEnabled = status?.kalshi_execution_enabled ?? false;
  const paperBalance = status?.paper_wallet_balance ?? 1000;
  const paperPnl = pnlSummaries.find((row) => row.follower_account_id === PAPER_FOLLOWER_ID) ?? null;
  const totalPnl = paperPnl?.total_pnl ?? null;
  const realizedPnl = paperPnl?.realized_pnl ?? null;
  const unrealizedPnl = paperPnl?.unrealized_pnl ?? null;
  const openPositions = paperPnl?.open_positions ?? 0;

  const totalIn  = walletMovements.filter(m => m.amount > 0).reduce((s, m) => s + m.amount, 0);
  const totalOut = walletMovements.filter(m => m.amount < 0).reduce((s, m) => s + m.amount, 0);
  const netCashFlow = totalIn + totalOut;

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
            <div className="space-y-3">
              <div className="grid grid-cols-1 md:grid-cols-[1.2fr_1fr_1fr_auto] gap-3">
                <div className="rounded bg-gray-900 border border-gray-800 px-4 py-3">
                  <p className="text-xs text-gray-500">Available Cash</p>
                  <p className="text-lg font-semibold text-yellow-300 mt-0.5">
                    {money.format(paperBalance)}
                  </p>
                  <p className="text-xs text-gray-600 mt-0.5">Liquid cash — excludes value locked in open positions</p>
                </div>
                <div className="rounded bg-gray-900 border border-gray-800 px-4 py-3">
                  <p className="text-xs text-gray-500">Total P&L</p>
                  <p className={`text-lg font-semibold mt-0.5 ${
                    (totalPnl ?? 0) >= 0 ? "text-green-400" : "text-red-400"
                  }`}>
                    {totalPnl == null ? "—" : formatSignedMoney(totalPnl)}
                  </p>
                  <p className="text-xs text-gray-600 mt-0.5">From closed and open positions</p>
                </div>
                <div className="rounded bg-gray-900 border border-gray-800 px-4 py-3">
                  <p className="text-xs text-gray-500">Open P&L</p>
                  <p className={`text-lg font-semibold mt-0.5 ${
                    (unrealizedPnl ?? 0) >= 0 ? "text-green-400" : "text-red-400"
                  }`}>
                    {unrealizedPnl == null ? "—" : formatSignedMoney(unrealizedPnl)}
                  </p>
                  <p className="text-xs text-gray-600 mt-0.5">Closed {realizedPnl == null ? "—" : formatSignedMoney(realizedPnl)}</p>
                </div>
                <div className="rounded bg-gray-900 border border-gray-800 px-4 py-3 flex flex-col justify-center gap-2">
                  <button
                    onClick={() => resetWallet.mutate()}
                    disabled={resetWallet.isPending}
                    className="rounded px-3 py-1.5 text-xs font-medium bg-gray-700 hover:bg-gray-600 text-gray-200 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    {resetWallet.isPending ? "Resetting..." : "Reset to $1,000"}
                  </button>
                  <p className="text-center text-xs text-gray-600">{openPositions} open positions</p>
                </div>
              </div>

              <div className="rounded bg-gray-900 border border-gray-800 overflow-hidden">
                <div className="flex items-center justify-between px-4 py-2 border-b border-gray-800">
                  <h4 className="text-xs font-medium uppercase tracking-wide text-gray-400">Paper Wallet Ledger</h4>
                  {walletMovements.length > 0 && (
                    <span className="rounded-full bg-gray-800 px-2 py-0.5 text-xs font-medium text-gray-300">
                      Latest {walletMovements.length}
                    </span>
                  )}
                </div>
                {walletMovements.length > 0 && (
                  <div className="grid grid-cols-3 divide-x divide-gray-800 border-b border-gray-800">
                    <div className="px-4 py-2">
                      <p className="text-[10px] text-gray-500 uppercase tracking-wide">Total In</p>
                      <p className="text-sm font-semibold text-green-400">+{money.format(totalIn)}</p>
                    </div>
                    <div className="px-4 py-2">
                      <p className="text-[10px] text-gray-500 uppercase tracking-wide">Total Out</p>
                      <p className="text-sm font-semibold text-red-400">{money.format(totalOut)}</p>
                    </div>
                    <div className="px-4 py-2">
                      <p className="text-[10px] text-gray-500 uppercase tracking-wide">Net Cash Flow</p>
                      <p className={`text-sm font-semibold ${netCashFlow >= 0 ? "text-green-400" : "text-red-400"}`}>
                        {formatSignedMoney(netCashFlow)}
                      </p>
                    </div>
                  </div>
                )}
                <div className="max-h-80 overflow-auto">
                  <table className="min-w-full text-xs">
                    <thead className="bg-gray-950 text-gray-500">
                      <tr>
                        <th className="px-3 py-2 text-left font-medium">Time</th>
                        <th className="px-3 py-2 text-left font-medium">Type</th>
                        <th className="px-3 py-2 text-right font-medium">Amount</th>
                        <th className="px-3 py-2 text-right font-medium">Balance</th>
                        <th className="px-3 py-2 text-left font-medium">Source</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-gray-800">
                      {walletMovements.slice(0, VISIBLE_LEDGER_ROWS).map((movement) => (
                        <tr key={movement.id} className="text-gray-300">
                          <td className="px-3 py-2 whitespace-nowrap text-gray-500">
                            {new Date(movement.created_at).toLocaleString()}
                          </td>
                          <td className="px-3 py-2 whitespace-nowrap">
                            {movementLabel(movement.movement_type)}
                          </td>
                          <td className={`px-3 py-2 text-right whitespace-nowrap font-medium ${
                            movement.amount >= 0 ? "text-emerald-300" : "text-red-300"
                          }`}>
                            {formatSignedMoney(movement.amount)}
                          </td>
                          <td className="px-3 py-2 text-right whitespace-nowrap text-gray-200">
                            {money.format(movement.balance_after)}
                          </td>
                          <td className="px-3 py-2 max-w-[180px] truncate text-gray-500">
                            {movementSource(movement)}
                          </td>
                        </tr>
                      ))}
                      {walletMovements.length === 0 && (
                        <tr>
                          <td colSpan={5} className="px-3 py-6 text-center text-gray-600">
                            No wallet movements recorded
                          </td>
                        </tr>
                      )}
                    </tbody>
                  </table>
                </div>
              </div>
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
