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
import { useMemo, type ReactNode } from "react";

const PAPER_FOLLOWER_ID = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";

function fmt(n: number | null | undefined, prefix = "$") {
  if (n == null || !Number.isFinite(n)) return "-";
  return `${prefix}${n.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
}

function fmtCompact(n: number | null | undefined) {
  if (n == null || !Number.isFinite(n)) return "-";
  const abs = Math.abs(n);
  const sign = n < 0 ? "-" : "";
  if (abs >= 1_000_000) return `${sign}$${(abs / 1_000_000).toFixed(1)}m`;
  if (abs >= 1_000) return `${sign}$${(abs / 1_000).toFixed(1)}k`;
  return fmt(n);
}

function pct(n: number | null | undefined) {
  if (n == null || !Number.isFinite(n)) return "-";
  return `${(n * 100).toFixed(1)}%`;
}

function cents(n: number | null | undefined) {
  if (n == null || !Number.isFinite(n)) return "-";
  return `${(n * 100).toFixed(1)}c`;
}

function timeLabel(value: string | null | undefined) {
  if (!value) return "-";
  return new Date(value).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

function ageLabel(value: string | null | undefined) {
  if (!value) return "-";
  const seconds = Math.max(0, Math.round((Date.now() - new Date(value).getTime()) / 1000));
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  return `${Math.round(minutes / 60)}h ago`;
}

function positionTotalPnl(p: any) {
  return Number(p.realized_pnl ?? 0) + Number(p.unrealized_pnl ?? 0);
}

function markPrice(p: any) {
  const contracts = Number(p.contracts);
  if (!Number.isFinite(contracts) || contracts === 0) return null;
  return Number(p.avg_price ?? 0) + Number(p.unrealized_pnl ?? 0) / contracts;
}

function isBuySide(side: unknown) {
  return String(side ?? "").toLowerCase() === "buy";
}

function moneyColor(n: number | null | undefined) {
  if (n == null || !Number.isFinite(n)) return "text-gray-100";
  if (n > 0) return "text-emerald-300";
  if (n < 0) return "text-rose-300";
  return "text-gray-100";
}

function sideColor(side: unknown) {
  return isBuySide(side)
    ? "border-emerald-500/30 bg-emerald-500/10 text-emerald-300"
    : "border-rose-500/30 bg-rose-500/10 text-rose-300";
}

function StatusPill({
  tone,
  children,
}: {
  tone: "green" | "yellow" | "red" | "blue" | "gray";
  children: ReactNode;
}) {
  const tones = {
    green: "border-emerald-500/30 bg-emerald-500/10 text-emerald-300",
    yellow: "border-amber-500/30 bg-amber-500/10 text-amber-300",
    red: "border-rose-500/30 bg-rose-500/10 text-rose-300",
    blue: "border-cyan-500/30 bg-cyan-500/10 text-cyan-300",
    gray: "border-gray-700 bg-gray-900 text-gray-300",
  };
  return (
    <span className={`inline-flex items-center rounded border px-2 py-1 text-xs font-semibold ${tones[tone]}`}>
      {children}
    </span>
  );
}

function MetricTile({
  label,
  value,
  sub,
  valueClass,
  accent = "bg-cyan-400",
}: {
  label: string;
  value: ReactNode;
  sub?: string;
  valueClass?: string;
  accent?: string;
}) {
  return (
    <div className="rounded-lg border border-gray-800 bg-gray-950 px-4 py-3.5">
      <div className="mb-2 flex items-center gap-2">
        <span className={`h-2 w-2 rounded-full ${accent}`} />
        <p className="text-[11px] font-semibold uppercase tracking-wide text-gray-500">{label}</p>
      </div>
      <div className={`text-2xl font-semibold leading-none tabular-nums ${valueClass ?? "text-gray-100"}`}>
        {value}
      </div>
      {sub && <p className="mt-2 text-xs text-gray-600">{sub}</p>}
    </div>
  );
}

function Panel({
  title,
  meta,
  children,
  className = "",
}: {
  title: string;
  meta?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={`rounded-lg border border-gray-800 bg-gray-950 ${className}`}>
      <div className="flex items-center justify-between gap-3 border-b border-gray-800 px-4 py-3">
        <h3 className="text-sm font-semibold text-gray-100">{title}</h3>
        {meta && <div className="text-xs text-gray-500">{meta}</div>}
      </div>
      {children}
    </section>
  );
}

function MiniPnlPath({ positions }: { positions: any[] }) {
  const points = useMemo(() => {
    const rows = [...positions]
      .filter((p) => p.updated_at)
      .sort((a, b) => new Date(a.updated_at).getTime() - new Date(b.updated_at).getTime())
      .slice(-18);
    let running = 0;
    return rows.map((p) => {
      running += positionTotalPnl(p);
      return running;
    });
  }, [positions]);

  if (points.length < 2) {
    return (
      <div className="flex h-20 items-center justify-center rounded border border-gray-800 bg-gray-950/70 text-xs text-gray-600">
        Waiting for more position updates
      </div>
    );
  }

  const width = 520;
  const height = 92;
  const min = Math.min(...points, 0);
  const max = Math.max(...points, 0);
  const range = max - min || 1;
  const x = (i: number) => (i / Math.max(points.length - 1, 1)) * width;
  const y = (v: number) => height - ((v - min) / range) * height;
  const d = points.map((v, i) => `${i === 0 ? "M" : "L"} ${x(i).toFixed(1)} ${y(v).toFixed(1)}`).join(" ");
  const last = points[points.length - 1];

  return (
    <svg className="h-20 w-full overflow-visible" viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none" aria-hidden="true">
      <line x1="0" x2={width} y1={y(0)} y2={y(0)} stroke="#374151" strokeDasharray="4 5" />
      <path d={d} fill="none" stroke={last >= 0 ? "#34d399" : "#fb7185"} strokeWidth="4" vectorEffect="non-scaling-stroke" />
    </svg>
  );
}

export function Dashboard() {
  const { data: status } = useSystemStatus();
  const { data: balances } = useAccountBalances();
  const { data: pnl } = usePnlSummary();
  const { data: masters } = useMasters();
  const { data: followers } = useFollowers();
  const { data: alerts } = useAlerts();
  const { data: restSignals } = useSignalsList(200);
  const { data: positions } = usePositions(1000);
  const liveSignals = useSignals();
  const toggleSystem = useToggleSystem();

  const allPositions = positions ?? [];
  const openPos = useMemo(
    () =>
      allPositions
        .filter((p: any) => Number(p.contracts) !== 0)
        .sort((a: any, b: any) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()),
    [allPositions]
  );

  const positionStats = allPositions.length > 0
    ? {
        totalPnl: allPositions.reduce((s: number, p: any) => s + positionTotalPnl(p), 0),
        unrealizedPnl: allPositions.reduce((s: number, p: any) => s + Number(p.unrealized_pnl ?? 0), 0),
        realizedPnl: allPositions.reduce((s: number, p: any) => s + Number(p.realized_pnl ?? 0), 0),
        openPositions: openPos.length,
      }
    : null;

  const pnlStats = pnl && pnl.length > 0
    ? {
        totalPnl: pnl.reduce((s, r) => s + r.total_pnl, 0),
        unrealizedPnl: pnl.reduce((s, r) => s + r.unrealized_pnl, 0),
        realizedPnl: pnl.reduce((s, r) => s + r.realized_pnl, 0),
        openPositions: pnl.reduce((s, r) => s + r.open_positions, 0),
        tradeCount: pnl.reduce((s, r) => s + r.trade_count, 0),
        weightedWins: pnl.reduce((s, r) => s + r.win_rate * r.trade_count, 0),
      }
    : null;

  const totalPnl = pnlStats?.totalPnl ?? positionStats?.totalPnl ?? null;
  const unrealizedPnl = pnlStats?.unrealizedPnl ?? positionStats?.unrealizedPnl ?? null;
  const realizedPnl = pnlStats?.realizedPnl ?? positionStats?.realizedPnl ?? null;
  const openPositions = pnlStats?.openPositions ?? positionStats?.openPositions ?? null;
  const winRate = pnlStats && pnlStats.tradeCount > 0 ? pnlStats.weightedWins / pnlStats.tradeCount : null;
  const tradeCount = pnlStats?.tradeCount ?? allPositions.length;
  const paperSummary = pnl?.find((row) => row.follower_account_id === PAPER_FOLLOWER_ID) ?? null;
  const paperPnl = paperSummary?.total_pnl ?? null;
  const paperCash = status?.paper_wallet_balance ?? null;

  const activeMasters = masters?.filter((m) => m.status === "active").length ?? 0;
  const activeFollowers = followers?.filter((f) => f.status === "active").length ?? 0;
  const openAlerts = alerts?.filter((a) => a.status === "open").length ?? status?.open_alert_count ?? 0;
  const exposure = openPos.reduce((s: number, p: any) => s + Number(p.exposure ?? 0), 0);
  const biggestOpenRisk = openPos.reduce((max: any | null, p: any) => {
    if (!max) return p;
    return Math.abs(positionTotalPnl(p)) > Math.abs(positionTotalPnl(max)) ? p : max;
  }, null);

  const recentSignals = useMemo(() => {
    const map = new Map<string, any>();
    for (const s of restSignals ?? []) map.set(s.id, s);
    for (const s of liveSignals) map.set(s.id, s);
    return Array.from(map.values())
      .sort((a, b) => new Date(b.detected_at).getTime() - new Date(a.detected_at).getTime())
      .slice(0, 10);
  }, [restSignals, liveSignals]);

  const latestSignalAt = recentSignals[0]?.detected_at ?? status?.last_signal_at ?? null;
  const recentOpens = recentSignals.filter((s: any) => s.position_effect === "open").length;
  const recentCloses = recentSignals.filter((s: any) => s.position_effect === "close").length;
  const kalshiLive = Boolean(status?.kalshi_execution_enabled);
  const systemLive = Boolean(status?.live_trading_enabled);

  return (
    <div className="-m-4 min-h-full bg-gray-900 text-gray-100 md:-m-6">
      <div className="border-b border-gray-800 bg-gray-950/80 px-4 py-5 md:px-6">
        <div className="flex flex-col gap-4 xl:flex-row xl:items-end xl:justify-between">
          <div>
            <div className="mb-3 flex flex-wrap items-center gap-2">
              <StatusPill tone={systemLive ? "green" : "yellow"}>
                {systemLive ? "Live trading on" : "Paper protected"}
              </StatusPill>
              <StatusPill tone={kalshiLive ? "green" : "gray"}>
                Kalshi {kalshiLive ? "live" : "paper"}
              </StatusPill>
              <StatusPill tone={openAlerts > 0 ? "yellow" : "green"}>
                {openAlerts} open alerts
              </StatusPill>
            </div>
            <h2 className="text-2xl font-semibold tracking-tight text-white">Dashboard</h2>
            <p className="mt-1 text-sm text-gray-500">
              {activeMasters} active masters, {activeFollowers} active followers, latest signal {ageLabel(latestSignalAt)}
            </p>
          </div>
          <button
            onClick={() => toggleSystem.mutate()}
            disabled={toggleSystem.isPending}
            className="h-10 w-fit rounded border border-gray-700 px-4 text-sm font-semibold text-gray-200 hover:border-gray-500 hover:bg-gray-900 disabled:cursor-not-allowed disabled:opacity-50"
          >
            Toggle Trading
          </button>
        </div>
      </div>

      <div className="space-y-5 px-4 py-5 md:px-6">
        <section className="grid gap-5 xl:grid-cols-[minmax(0,1.35fr)_minmax(360px,0.65fr)]">
          <div className="rounded-lg border border-gray-800 bg-gray-950 p-5">
            <div className="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
              <div>
                <p className="text-xs font-semibold uppercase tracking-wide text-gray-500">Total P&L</p>
                <div className={`mt-2 text-5xl font-semibold leading-none tracking-tight tabular-nums ${moneyColor(totalPnl)}`}>
                  {fmtCompact(totalPnl)}
                </div>
                <p className="mt-3 text-sm text-gray-500">
                  {fmt(unrealizedPnl)} open, {fmt(realizedPnl)} closed
                </p>
              </div>
              <div className="grid min-w-[280px] grid-cols-2 gap-x-6 gap-y-3 text-sm">
                <div>
                  <p className="text-xs uppercase tracking-wide text-gray-600">Paper cash</p>
                  <p className="mt-1 font-semibold tabular-nums text-gray-100">{fmt(paperCash)}</p>
                </div>
                <div>
                  <p className="text-xs uppercase tracking-wide text-gray-600">Paper P&L</p>
                  <p className={`mt-1 font-semibold tabular-nums ${moneyColor(paperPnl)}`}>{fmt(paperPnl)}</p>
                </div>
                <div>
                  <p className="text-xs uppercase tracking-wide text-gray-600">Open exposure</p>
                  <p className="mt-1 font-semibold tabular-nums text-gray-100">{fmt(exposure)}</p>
                </div>
                <div>
                  <p className="text-xs uppercase tracking-wide text-gray-600">Last signal</p>
                  <p className="mt-1 font-semibold tabular-nums text-gray-100">{timeLabel(latestSignalAt)}</p>
                </div>
              </div>
            </div>
            <div className="mt-5">
              <MiniPnlPath positions={allPositions} />
            </div>
          </div>

          <div className="rounded-lg border border-gray-800 bg-gray-950 p-5">
            <div className="mb-4 flex items-center justify-between">
              <h3 className="text-sm font-semibold text-gray-100">Execution Lane</h3>
              <span className="h-2 w-2 rounded-full bg-emerald-400 shadow-[0_0_18px_rgba(52,211,153,0.8)]" />
            </div>
            <div className="space-y-3">
              {[
                ["Master poll", "1s", "text-cyan-300"],
                ["Signal normalize", "250ms", "text-emerald-300"],
                ["Risk evaluate", "250ms", "text-amber-300"],
                ["Paper execute", "250ms", "text-violet-300"],
                ["PnL refresh", "1s", "text-rose-300"],
              ].map(([label, value, color]) => (
                <div key={label} className="grid grid-cols-[1fr_auto] items-center gap-4 border-b border-gray-800/70 pb-3 last:border-0 last:pb-0">
                  <span className="text-sm text-gray-400">{label}</span>
                  <span className={`font-mono text-sm font-semibold ${color}`}>{value}</span>
                </div>
              ))}
            </div>
          </div>
        </section>

        <section className="grid grid-cols-2 gap-4 lg:grid-cols-5">
          <MetricTile label="Open positions" value={openPositions ?? "-"} sub={`${tradeCount} tracked trades`} accent="bg-cyan-400" />
          <MetricTile label="Win rate" value={pct(winRate)} sub={`${recentOpens} opens / ${recentCloses} closes`} accent="bg-emerald-400" />
          <MetricTile label="Unrealized" value={fmt(unrealizedPnl)} valueClass={moneyColor(unrealizedPnl)} sub="Open mark-to-market" accent="bg-amber-400" />
          <MetricTile label="Realized" value={fmt(realizedPnl)} valueClass={moneyColor(realizedPnl)} sub="Closed settlements" accent="bg-violet-400" />
          <MetricTile
            label="Largest swing"
            value={biggestOpenRisk ? fmt(positionTotalPnl(biggestOpenRisk)) : "-"}
            valueClass={moneyColor(biggestOpenRisk ? positionTotalPnl(biggestOpenRisk) : null)}
            sub={biggestOpenRisk?.outcome ? String(biggestOpenRisk.outcome).toUpperCase() : "No open risk"}
            accent="bg-rose-400"
          />
        </section>

        <section className="grid gap-5 2xl:grid-cols-[minmax(0,0.92fr)_minmax(720px,1.08fr)]">
          <Panel
            title="Live Signals"
            meta={latestSignalAt ? `Latest ${timeLabel(latestSignalAt)}` : "Waiting"}
            className="overflow-hidden"
          >
            {recentSignals.length === 0 ? (
              <div className="px-4 py-12 text-center text-sm text-gray-600">No signals yet</div>
            ) : (
              <div className="divide-y divide-gray-800">
                {recentSignals.map((s: any) => (
                  <div key={s.id} className="grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3 px-4 py-3 hover:bg-gray-900/80">
                    <span className={`rounded border px-2 py-1 text-xs font-semibold ${sideColor(s.side)}`}>
                      {String(s.side ?? "-").toUpperCase()}
                    </span>
                    <div className="min-w-0">
                      <p className="truncate text-sm font-medium text-gray-100" title={s.market_title || s.outcome || ""}>
                        {s.market_title || s.outcome || "-"}
                      </p>
                      <p className="mt-0.5 text-xs text-gray-600">
                        {s.outcome ?? "-"} / {s.position_effect ?? "-"} / {s.status ?? "-"}
                      </p>
                    </div>
                    <div className="text-right">
                      <p className="font-mono text-sm text-gray-200">{s.entry_price != null ? cents(Number(s.entry_price)) : "-"}</p>
                      <p className="text-xs text-gray-600">{ageLabel(s.detected_at)}</p>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </Panel>

          <Panel
            title="Open Position Blotter"
            meta={`${openPos.length} open / ${fmt(exposure)} exposure`}
            className="overflow-hidden"
          >
            {openPos.length === 0 ? (
              <div className="px-4 py-12 text-center text-sm text-gray-600">No open positions</div>
            ) : (
              <div className="max-h-[560px] overflow-auto">
                <table className="w-full min-w-[760px] text-sm">
                  <thead className="sticky top-0 z-10 bg-gray-950">
                    <tr className="border-b border-gray-800 text-left text-[11px] uppercase tracking-wide text-gray-500">
                      <th className="px-4 py-3">Market</th>
                      <th className="px-4 py-3">Outcome</th>
                      <th className="px-4 py-3 text-right">Contracts</th>
                      <th className="px-4 py-3 text-right">Avg</th>
                      <th className="px-4 py-3 text-right">Mark</th>
                      <th className="px-4 py-3 text-right">Open P&L</th>
                      <th className="px-4 py-3 text-right">Total</th>
                      <th className="px-4 py-3 text-right">Updated</th>
                    </tr>
                  </thead>
                  <tbody>
                    {openPos.slice(0, 18).map((p: any) => {
                      const total = positionTotalPnl(p);
                      return (
                        <tr key={p.id} className="border-b border-gray-800/70 hover:bg-gray-900/80">
                          <td className="max-w-[280px] px-4 py-3">
                            <p className="truncate font-medium text-gray-100" title={p.market_title || p.market_id}>
                              {p.market_title || p.market_id || "-"}
                            </p>
                            <p className="mt-0.5 text-xs text-gray-600">{p.platform}</p>
                          </td>
                          <td className="px-4 py-3 text-xs uppercase text-gray-300">{p.outcome}</td>
                          <td className="px-4 py-3 text-right font-mono text-gray-300">{Number(p.contracts).toFixed(2)}</td>
                          <td className="px-4 py-3 text-right font-mono text-gray-300">{cents(Number(p.avg_price))}</td>
                          <td className="px-4 py-3 text-right font-mono text-gray-300">{cents(markPrice(p))}</td>
                          <td className={`px-4 py-3 text-right font-mono font-semibold ${moneyColor(Number(p.unrealized_pnl))}`}>
                            {fmt(Number(p.unrealized_pnl))}
                          </td>
                          <td className={`px-4 py-3 text-right font-mono font-semibold ${moneyColor(total)}`}>
                            {fmt(total)}
                          </td>
                          <td className="px-4 py-3 text-right font-mono text-xs text-gray-500">{timeLabel(p.updated_at)}</td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </Panel>
        </section>

        <section className="grid gap-5 xl:grid-cols-3">
          <Panel title="Account Lanes" meta="Balances" className="xl:col-span-2">
            <div className="grid gap-0 divide-y divide-gray-800 md:grid-cols-3 md:divide-x md:divide-y-0">
              <div className="p-4">
                <p className="text-xs font-semibold uppercase tracking-wide text-gray-500">Kalshi slave</p>
                <p className="mt-2 text-2xl font-semibold tabular-nums text-gray-100">
                  {kalshiLive && balances?.kalshi_balance != null ? fmt(balances.kalshi_balance) : fmt(paperCash)}
                </p>
                <p className="mt-1 text-xs text-gray-600">{kalshiLive ? "Production balance" : "Paper wallet cash"}</p>
              </div>
              <div className="p-4">
                <p className="text-xs font-semibold uppercase tracking-wide text-gray-500">Polymarket</p>
                <p className="mt-2 text-2xl font-semibold tabular-nums text-gray-100">
                  {balances?.polymarket_balance != null ? fmt(balances.polymarket_balance) : "-"}
                </p>
                <p className="mt-1 text-xs text-gray-600">{balances?.polymarket_error ?? "CLOB wallet"}</p>
              </div>
              <div className="p-4">
                <p className="text-xs font-semibold uppercase tracking-wide text-gray-500">Follower result</p>
                <p className={`mt-2 text-2xl font-semibold tabular-nums ${moneyColor(paperPnl)}`}>{fmt(paperPnl)}</p>
                <p className="mt-1 text-xs text-gray-600">{paperSummary?.open_positions ?? 0} open positions</p>
              </div>
            </div>
          </Panel>

          <Panel title="System Pulse" meta={status?.worker_mode ?? "worker"}>
            <div className="space-y-3 p-4">
              <div className="flex items-center justify-between gap-4">
                <span className="text-sm text-gray-500">Masters</span>
                <span className="font-mono text-sm text-gray-100">{activeMasters}/{masters?.length ?? 0}</span>
              </div>
              <div className="flex items-center justify-between gap-4">
                <span className="text-sm text-gray-500">Followers</span>
                <span className="font-mono text-sm text-gray-100">{activeFollowers}/{followers?.length ?? 0}</span>
              </div>
              <div className="flex items-center justify-between gap-4">
                <span className="text-sm text-gray-500">Signals shown</span>
                <span className="font-mono text-sm text-gray-100">{recentSignals.length}</span>
              </div>
              <div className="flex items-center justify-between gap-4">
                <span className="text-sm text-gray-500">Open alerts</span>
                <span className={`font-mono text-sm font-semibold ${openAlerts > 0 ? "text-amber-300" : "text-emerald-300"}`}>{openAlerts}</span>
              </div>
            </div>
          </Panel>
        </section>
      </div>
    </div>
  );
}
