import {
  useSystemStatus,
  useMasters,
  useFollowers,
  useAlerts,
  useAccountBalances,
  usePnlSummary,
  useSignalsList,
  usePositions,
  useToggleSystem,
} from "@/hooks/useApi";
import { useSignals } from "@/hooks/useSignals";
import { useMemo } from "react";

function fmt(n: number | null | undefined, prefix = "$") {
  if (n == null) return "—";
  return `${prefix}${n.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
}

function pct(n: number | null | undefined) {
  if (n == null) return "—";
  return `${(n * 100).toFixed(1)}%`;
}

function cents(n: number | null | undefined) {
  if (n == null || !Number.isFinite(n)) return "—";
  return `${(n * 100).toFixed(1)}¢`;
}

function positionTotalPnl(p: any) {
  return Number(p.realized_pnl ?? 0) + Number(p.unrealized_pnl ?? 0);
}

function markPrice(p: any) {
  const contracts = Number(p.contracts);
  if (!Number.isFinite(contracts) || contracts === 0) return null;
  return Number(p.avg_price ?? 0) + Number(p.unrealized_pnl ?? 0) / contracts;
}

function StatCard({
  label,
  value,
  sub,
  color,
}: {
  label: string;
  value: React.ReactNode;
  sub?: string;
  color?: string;
}) {
  return (
    <div className="rounded-lg border border-gray-800 bg-gray-950 p-4">
      <p className="text-xs text-gray-500 uppercase tracking-wide mb-1">{label}</p>
      <p className={`text-2xl font-semibold ${color ?? "text-white"}`}>{value}</p>
      {sub && <p className="text-xs text-gray-500 mt-1">{sub}</p>}
    </div>
  );
}

export function Dashboard() {
  const { data: status } = useSystemStatus();
  const { data: balances } = useAccountBalances();
  const { data: pnl } = usePnlSummary();
  const { data: masters } = useMasters();
  const { data: followers } = useFollowers();
  const { data: alerts } = useAlerts();
  const { data: restSignals } = useSignalsList();
  const { data: positions } = usePositions();
  const liveSignals = useSignals();
  const toggleSystem = useToggleSystem();

  const allPositions = positions ?? [];
  const openPos = allPositions.filter((p: any) => Number(p.contracts) !== 0);
  const positionStats = allPositions.length > 0 ? {
    totalPnl: allPositions.reduce((s: number, p: any) => s + positionTotalPnl(p), 0),
    unrealizedPnl: allPositions.reduce((s: number, p: any) => s + Number(p.unrealized_pnl ?? 0), 0),
    realizedPnl: allPositions.reduce((s: number, p: any) => s + Number(p.realized_pnl ?? 0), 0),
    openPositions: openPos.length,
  } : null;

  const totalPnl = positionStats?.totalPnl ?? pnl?.reduce((s, r) => s + r.total_pnl, 0) ?? null;
  const unrealizedPnl = positionStats?.unrealizedPnl ?? pnl?.reduce((s, r) => s + r.unrealized_pnl, 0) ?? null;
  const realizedPnl = positionStats?.realizedPnl ?? pnl?.reduce((s, r) => s + r.realized_pnl, 0) ?? null;
  const openPositions = positionStats?.openPositions ?? pnl?.reduce((s, r) => s + r.open_positions, 0) ?? null;
  const winRate = pnl && pnl.length > 0 ? pnl.reduce((s, r) => s + r.win_rate, 0) / pnl.length : null;
  const paperCash = status?.paper_wallet_balance ?? null;
  const paperEquity = totalPnl != null ? 1000 + totalPnl : paperCash;

  const activeMasters = masters?.filter((m) => m.status === "active").length ?? 0;
  const activeFollowers = followers?.filter((f) => f.status === "active").length ?? 0;
  const openAlerts = alerts?.filter((a) => a.status === "open").length ?? status?.open_alert_count ?? 0;

  // Merge live SSE signals on top of REST, deduped by id
  const recentSignals = useMemo(() => {
    const map = new Map<string, any>();
    for (const s of restSignals ?? []) map.set(s.id, s);
    for (const s of liveSignals) map.set(s.id, s);
    return Array.from(map.values())
      .sort((a, b) => new Date(b.detected_at).getTime() - new Date(a.detected_at).getTime())
      .slice(0, 8);
  }, [restSignals, liveSignals]);

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold">Dashboard</h2>
        <div className="flex items-center gap-3">
          <span className={`text-sm font-medium px-3 py-1 rounded-full ${
            status?.live_trading_enabled
              ? "bg-green-900/50 text-green-300 border border-green-800"
              : "bg-yellow-900/50 text-yellow-300 border border-yellow-800"
          }`}>
            {status?.live_trading_enabled ? "Live Trading ON" : "Live Trading OFF"}
          </span>
          <button
            onClick={() => toggleSystem.mutate()}
            disabled={toggleSystem.isPending}
            className="text-xs px-3 py-1 rounded border border-gray-700 text-gray-400 hover:border-gray-500 hover:text-gray-200 disabled:opacity-50"
          >
            Toggle
          </button>
        </div>
      </div>

      {/* Account Balances */}
      <div>
        <h3 className="text-sm font-medium text-gray-400 mb-3">Account Balances</h3>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {/* Kalshi slave account — live or paper */}
          <div className={`rounded-lg border p-4 flex items-center gap-4 ${
            status?.kalshi_execution_enabled
              ? "border-green-800 bg-green-950/20"
              : "border-yellow-800/50 bg-yellow-950/10"
          }`}>
            <div className={`w-10 h-10 rounded-lg flex items-center justify-center font-bold text-sm ${
              status?.kalshi_execution_enabled
                ? "bg-green-900/40 border border-green-800 text-green-400"
                : "bg-yellow-900/40 border border-yellow-800 text-yellow-400"
            }`}>K</div>
            <div>
              <div className="flex items-center gap-2">
                <p className="text-xs text-gray-500 uppercase tracking-wide">Kalshi Slave</p>
                <span className={`text-xs px-1.5 py-0.5 rounded font-medium ${
                  status?.kalshi_execution_enabled
                    ? "bg-green-900 text-green-300"
                    : "bg-yellow-900/60 text-yellow-300"
                }`}>
                  {status?.kalshi_execution_enabled ? "LIVE" : "PAPER"}
                </span>
              </div>
              <p className="text-2xl font-semibold text-white mt-0.5">
                {status?.kalshi_execution_enabled
                  ? (balances?.kalshi_balance != null
                      ? fmt(balances.kalshi_balance)
                      : <span className="text-sm text-red-400">{balances?.kalshi_error ?? "—"}</span>)
                  : fmt(paperEquity)
                }
              </p>
              <p className="text-xs text-gray-600">
                {status?.kalshi_execution_enabled ? "USDC · production" : `Cash available ${fmt(paperCash)}`}
              </p>
            </div>
          </div>

          {/* Polymarket master */}
          <div className="rounded-lg border border-gray-800 bg-gray-950 p-4 flex items-center gap-4">
            <div className="w-10 h-10 rounded-lg bg-purple-900/40 border border-purple-800 flex items-center justify-center text-purple-400 font-bold text-sm">P</div>
            <div>
              <p className="text-xs text-gray-500 uppercase tracking-wide">Polymarket</p>
              <p className="text-2xl font-semibold text-white mt-0.5">
                {balances?.polymarket_balance != null
                  ? fmt(balances.polymarket_balance)
                  : (balances?.polymarket_error
                      ? <span className="text-sm text-red-400">{balances.polymarket_error}</span>
                      : "—")}
              </p>
              <p className="text-xs text-gray-600">USDC · CLOB wallet</p>
            </div>
          </div>

          {/* P&L quick stat */}
          <div className="rounded-lg border border-gray-800 bg-gray-950 p-4 flex items-center gap-4">
            <div className="w-10 h-10 rounded-lg bg-indigo-900/40 border border-indigo-800 flex items-center justify-center text-indigo-400 font-bold text-sm">$</div>
            <div>
              <p className="text-xs text-gray-500 uppercase tracking-wide">Total P&L</p>
              <p className={`text-2xl font-semibold mt-0.5 ${
                (totalPnl ?? 0) >= 0 ? "text-green-400" : "text-red-400"
              }`}>
                {fmt(totalPnl)}
              </p>
              <p className="text-xs text-gray-600">{openPositions ?? 0} open positions</p>
            </div>
          </div>
        </div>
      </div>

      {/* P&L Row */}
      <div>
        <h3 className="text-sm font-medium text-gray-400 mb-3">P&L Summary</h3>
        <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
          <StatCard
            label="Total P&L"
            value={fmt(totalPnl)}
            color={totalPnl != null ? (totalPnl >= 0 ? "text-green-400" : "text-red-400") : "text-white"}
          />
          <StatCard
            label="Closed P&L"
            value={fmt(realizedPnl)}
            color={realizedPnl != null ? (realizedPnl >= 0 ? "text-green-400" : "text-red-400") : "text-white"}
            sub="updates after exits"
          />
          <StatCard
            label="Open P&L"
            value={fmt(unrealizedPnl)}
            color={unrealizedPnl != null ? (unrealizedPnl >= 0 ? "text-green-400" : "text-red-400") : "text-white"}
            sub="moves while positions are open"
          />
          <StatCard label="Win Rate" value={pct(winRate)} />
          <StatCard label="Open Positions" value={openPositions ?? "—"} />
        </div>
      </div>

      {/* System Stats Row */}
      <div>
        <h3 className="text-sm font-medium text-gray-400 mb-3">System</h3>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <StatCard
            label="Active Masters"
            value={activeMasters}
            sub={`${masters?.length ?? 0} total`}
          />
          <StatCard
            label="Active Followers"
            value={activeFollowers}
            sub={`${followers?.length ?? 0} total`}
          />
          <StatCard
            label="Open Alerts"
            value={openAlerts}
            color={Number(openAlerts) > 0 ? "text-yellow-400" : "text-white"}
          />
          <StatCard
            label="Last Signal"
            value={status?.last_signal_at ? new Date(status.last_signal_at).toLocaleTimeString() : "—"}
            sub={status?.worker_mode}
          />
        </div>
      </div>

      {/* Two-column bottom section */}
      <div className="grid grid-cols-1 xl:grid-cols-2 gap-6">
        {/* Recent Signals — real-time */}
        <div className="rounded-lg border border-gray-800 bg-gray-950">
          <div className="px-4 py-3 border-b border-gray-800 flex items-center gap-2">
            <h3 className="text-sm font-semibold text-gray-300">Recent Signals</h3>
            {liveSignals.length > 0 && (
              <span className="flex items-center gap-1 text-xs text-green-400">
                <span className="h-1.5 w-1.5 rounded-full bg-green-400 animate-pulse" />
                Live
              </span>
            )}
          </div>
          {recentSignals.length === 0 ? (
            <p className="px-4 py-6 text-center text-gray-600 text-sm">No signals yet</p>
          ) : (
            <table className="w-full text-sm">
              <thead>
                <tr className="text-left text-gray-500 text-xs uppercase border-b border-gray-800">
                  <th className="px-4 py-2">Market / Pair</th>
                  <th className="px-4 py-2">Side / Outcome</th>
                  <th className="px-4 py-2">Effect</th>
                  <th className="px-4 py-2">Price</th>
                  <th className="px-4 py-2">Status</th>
                  <th className="px-4 py-2">Time</th>
                </tr>
              </thead>
              <tbody>
                {recentSignals.map((s: any) => (
                  <tr key={s.id} className="border-b border-gray-800/50 hover:bg-gray-900">
                    <td className="px-4 py-2 max-w-[160px]">
                      {s.market_title ? (
                        <span className="truncate block text-gray-200 text-xs" title={s.market_title}>
                          {s.market_title}
                        </span>
                      ) : (
                        <span className="text-gray-400 text-xs">{s.outcome || "—"}</span>
                      )}
                    </td>
                    <td className="px-4 py-2">
                      <div className="flex flex-col gap-0.5">
                        <span className={`px-1.5 py-0.5 rounded text-xs font-medium w-fit ${s.side === "buy" ? "bg-green-900/60 text-green-300" : "bg-red-900/60 text-red-300"}`}>
                          {s.side?.toUpperCase()}
                        </span>
                        {s.outcome && (
                          <span className="px-1.5 py-0.5 rounded text-xs w-fit bg-gray-800 text-gray-300 capitalize">
                            {s.outcome}
                          </span>
                        )}
                      </div>
                    </td>
                    <td className="px-4 py-2">
                      <span className={`px-1.5 py-0.5 rounded text-xs ${
                        s.position_effect === "open"
                          ? "bg-blue-900/50 text-blue-300"
                          : s.position_effect === "close"
                          ? "bg-orange-900/50 text-orange-300"
                          : "bg-gray-800 text-gray-500"
                      }`}>
                        {s.position_effect?.toUpperCase() ?? "—"}
                      </span>
                    </td>
                    <td className="px-4 py-2 text-gray-300 text-xs">{s.entry_price != null ? `${(s.entry_price * 100).toFixed(0)}¢` : "—"}</td>
                    <td className="px-4 py-2">
                      <span className={`px-1.5 py-0.5 rounded text-xs ${
                        s.status === "approved" ? "bg-green-900/60 text-green-300" :
                        s.status === "new" ? "bg-blue-900/60 text-blue-300" :
                        "bg-gray-800 text-gray-400"
                      }`}>{s.status}</span>
                    </td>
                    <td className="px-4 py-2 text-gray-500 text-xs">{new Date(s.detected_at).toLocaleTimeString()}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        {/* Open Positions */}
        <div className="rounded-lg border border-gray-800 bg-gray-950 overflow-x-auto">
          <div className="px-4 py-3 border-b border-gray-800">
            <h3 className="text-sm font-semibold text-gray-300">
              Open Positions
              {openPos.length > 0 && (
                <span className="ml-2 text-xs font-normal text-gray-500">({openPos.length})</span>
              )}
            </h3>
          </div>
          {openPos.length === 0 ? (
            <p className="px-4 py-6 text-center text-gray-600 text-sm">No open positions</p>
          ) : (
            <table className="w-full text-sm">
              <thead>
                <tr className="text-left text-gray-500 text-xs uppercase border-b border-gray-800">
                  <th className="px-4 py-2">Market</th>
                  <th className="px-4 py-2">Outcome</th>
                  <th className="px-4 py-2">Contracts</th>
                  <th className="px-4 py-2">Avg Price</th>
                  <th className="px-4 py-2">Mark</th>
                  <th className="px-4 py-2">Open P&L</th>
                  <th className="px-4 py-2">Total P&L</th>
                  <th className="px-4 py-2">Updated</th>
                </tr>
              </thead>
              <tbody>
                {openPos.map((p: any) => (
                  <tr key={p.id} className="border-b border-gray-800/50 hover:bg-gray-900">
                    <td className="px-4 py-2 max-w-[160px]">
                      <span className="truncate block text-gray-200 text-xs" title={p.market_title || p.market_id}>
                        {p.market_title || p.market_id?.slice(0, 10) + "…" || "—"}
                      </span>
                    </td>
                    <td className="px-4 py-2 text-gray-300 text-xs">{p.outcome}</td>
                    <td className="px-4 py-2 text-gray-300 text-xs">{Number(p.contracts).toFixed(2)}</td>
                    <td className="px-4 py-2 text-gray-300 text-xs">{cents(Number(p.avg_price))}</td>
                    <td className="px-4 py-2 text-gray-300 text-xs">{cents(markPrice(p))}</td>
                    <td className={`px-4 py-2 text-xs font-medium ${Number(p.unrealized_pnl) >= 0 ? "text-green-400" : "text-red-400"}`}>
                      {fmt(Number(p.unrealized_pnl))}
                    </td>
                    <td className={`px-4 py-2 text-xs font-medium ${positionTotalPnl(p) >= 0 ? "text-green-400" : "text-red-400"}`}>
                      {fmt(positionTotalPnl(p))}
                    </td>
                    <td className="px-4 py-2 text-gray-500 text-xs whitespace-nowrap">
                      {new Date(p.updated_at).toLocaleTimeString()}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </div>
    </div>
  );
}
