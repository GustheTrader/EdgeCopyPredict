import { useState, useMemo, useRef, useEffect, type ReactNode } from "react";
import { useOrders, usePositions, usePaperWalletMovements, usePnlSummary } from "@/hooks/useApi";
import { Pagination } from "@/components/ui/Pagination";

const PER_PAGE = 20;
const HISTORY_LIMIT = 1000;
const PAPER_FOLLOWER_ID = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
type Tab = "orders" | "positions";

function fmt(n: number) {
  return `$${n.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
}

function fmtCompact(n: number) {
  const abs = Math.abs(n);
  const sign = n < 0 ? "-" : "";
  if (abs >= 1000) return `${sign}$${(abs / 1000).toFixed(1)}k`;
  return fmt(n);
}

function dayKey(dateStr: string) {
  const d = new Date(dateStr);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

function dayLabel(key: string) {
  return new Date(`${key}T12:00:00`).toLocaleDateString("en-US", { month: "short", day: "numeric" });
}

function moneyColor(n: number) {
  if (n > 0) return "text-emerald-300";
  if (n < 0) return "text-rose-300";
  return "text-gray-100";
}

function positionPnl(position: any) {
  return Number(position.realized_pnl ?? 0) + Number(position.unrealized_pnl ?? 0);
}

function EmptyChart({ height, label }: { height: number; label: string }) {
  return (
    <div className="flex items-center justify-center text-sm text-gray-600" style={{ minHeight: height }}>
      {label}
    </div>
  );
}

// ── Stat Card ─────────────────────────────────────────────────────────────────
function StatCard({
  label, value, sub, valueColor, accent = "bg-sky-400",
}: {
  label: string; value: string; sub?: string; valueColor?: string; accent?: string;
}) {
  return (
    <div className="min-w-[160px] flex-1 rounded-lg border border-gray-800 bg-gray-950 px-4 py-3.5">
      <div className="mb-2 flex items-center gap-2">
        <span className={`h-2 w-2 rounded-full ${accent}`} />
        <p className="text-[11px] font-semibold uppercase tracking-wide text-gray-500">{label}</p>
      </div>
      <p className={`text-2xl font-semibold tabular-nums leading-none ${valueColor ?? "text-gray-100"}`}>{value}</p>
      {sub && <p className="mt-1.5 text-xs text-gray-600">{sub}</p>}
    </div>
  );
}

function ChartPanel({
  title,
  subtitle,
  metric,
  metricClass,
  className = "",
  children,
}: {
  title: string;
  subtitle: string;
  metric?: string;
  metricClass?: string;
  className?: string;
  children: ReactNode;
}) {
  return (
    <section className={`rounded-lg border border-gray-800 bg-gray-950 p-4 shadow-[0_12px_30px_rgba(0,0,0,0.18)] ${className}`}>
      <div className="mb-4 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h3 className="text-sm font-semibold text-gray-100">{title}</h3>
          <p className="mt-1 text-xs text-gray-500">{subtitle}</p>
        </div>
        {metric && (
          <div className={`text-right text-xl font-semibold tabular-nums ${metricClass ?? "text-gray-100"}`}>
            {metric}
          </div>
        )}
      </div>
      {children}
    </section>
  );
}

// ── Cumulative P&L chart ──────────────────────────────────────────────────────
function CumulativePnlChart({ positions }: { positions: any[] }) {
  const dailyRows = useMemo(() => {
    const map = new Map<string, number>();
    for (const position of positions) {
      if (!position.updated_at) continue;
      const amount = positionPnl(position);
      if (!Number.isFinite(amount)) continue;
      const key = dayKey(position.updated_at);
      map.set(key, (map.get(key) ?? 0) + amount);
    }

    let cumulative = 0;
    return Array.from(map.entries())
      .sort((a, b) => a[0].localeCompare(b[0]))
      .map(([key, amount]) => {
        cumulative += amount;
        return {
          key,
          t: new Date(`${key}T12:00:00`).getTime(),
          amount,
          cumulative,
        };
      });
  }, [positions]);

  if (dailyRows.length === 0) return <EmptyChart height={220} label="No position P&L data in selected range" />;

  const oneDay = 86_400_000;
  const series = [
    { t: dailyRows[0].t - oneDay, v: 0 },
    ...dailyRows.map((row) => ({ t: row.t, v: row.cumulative })),
  ];

  const W = 900;
  const H = 220;
  const PAD = { top: 16, right: 18, bottom: 26, left: 64 };
  const iW = W - PAD.left - PAD.right;
  const lineH = 132;
  const barTop = 168;
  const barBase = 194;
  const barMaxH = 26;

  const times = series.map((p) => p.t);
  const values = series.map((p) => p.v);
  const minT = times[0];
  const maxT = times[times.length - 1];
  const rawMin = Math.min(...values);
  const rawMax = Math.max(...values);
  const vPad = Math.max(Math.abs(rawMax - rawMin) * 0.16, 2);
  const minV = Math.min(0, rawMin) - vPad;
  const maxV = Math.max(0, rawMax) + vPad;
  const rangeV = maxV - minV;
  const rangeT = maxT - minT || 1;

  const ix = (t: number) => PAD.left + ((t - minT) / rangeT) * iW;
  const iy = (v: number) => PAD.top + (1 - (v - minV) / rangeV) * lineH;

  const coords = series.map((p) => ({ x: ix(p.t), y: iy(p.v), v: p.v, t: p.t }));
  const zeroY = iy(0);
  const lastV = values[values.length - 1];
  const lineColor = lastV >= 0 ? "#10b981" : "#f43f5e";

  const linePath = coords
    .map((c, i) => {
      if (i === 0) return `M${c.x.toFixed(1)},${c.y.toFixed(1)}`;
      return `L${c.x.toFixed(1)},${c.y.toFixed(1)}`;
    })
    .join(" ");
  const areaPath = `${linePath} L${coords[coords.length - 1].x.toFixed(1)},${zeroY.toFixed(1)} L${coords[0].x.toFixed(1)},${zeroY.toFixed(1)} Z`;

  function niceStep(range: number, n: number) {
    const rough = range / n;
    const mag = Math.pow(10, Math.floor(Math.log10(Math.max(rough, 0.01))));
    for (const f of [1, 2, 2.5, 5, 10]) {
      if (f * mag >= rough) return f * mag;
    }
    return rough;
  }
  const step = niceStep(rangeV, 4);
  const yStart = Math.ceil(minV / step) * step;
  const yTicks: { y: number; label: string }[] = [];
  for (let v = yStart; v <= maxV + step * 0.01; v += step) {
    const snapped = Math.abs(v) < step * 0.01 ? 0 : v;
    yTicks.push({ y: iy(snapped), label: snapped === 0 ? "$0" : fmtCompact(snapped) });
  }

  const xTicks = [dailyRows[0], dailyRows[Math.floor(dailyRows.length / 2)], dailyRows[dailyRows.length - 1]]
    .filter((row, idx, arr) => row && arr.findIndex((other) => other.key === row.key) === idx)
    .map((row) => ({ x: ix(row.t), label: dayLabel(row.key) }));

  const peak = Math.max(0, ...dailyRows.map((row) => row.cumulative));
  const bestDay = dailyRows.reduce((a, b) => (b.amount > a.amount ? b : a), dailyRows[0]);
  const worstDay = dailyRows.reduce((a, b) => (b.amount < a.amount ? b : a), dailyRows[0]);
  const winDays = dailyRows.filter((row) => row.amount > 0).length;
  let runningPeak = 0;
  let maxDrawdown = 0;
  for (const row of dailyRows) {
    runningPeak = Math.max(runningPeak, row.cumulative);
    maxDrawdown = Math.max(maxDrawdown, runningPeak - row.cumulative);
  }

  const last = coords[coords.length - 1];
  const maxDailyAbs = Math.max(1, ...dailyRows.map((row) => Math.abs(row.amount)));
  const barSlot = iW / Math.max(1, dailyRows.length);
  const barW = Math.max(2, Math.min(10, barSlot * 0.58));

  const statRows = [
    { label: "Peak", value: fmt(peak), color: moneyColor(peak) },
    { label: "Drawdown", value: fmt(maxDrawdown), color: maxDrawdown > 0 ? "text-rose-300" : "text-gray-200" },
    { label: "Best day", value: fmt(bestDay.amount), sub: dayLabel(bestDay.key), color: moneyColor(bestDay.amount) },
    { label: "Worst day", value: fmt(worstDay.amount), sub: dayLabel(worstDay.key), color: moneyColor(worstDay.amount) },
    { label: "Win days", value: `${winDays}/${dailyRows.length}`, sub: `${((winDays / dailyRows.length) * 100).toFixed(0)}% positive`, color: "text-gray-100" },
  ];

  return (
    <div className="space-y-3">
      <svg
        viewBox={`0 0 ${W} ${H}`}
        className="block h-auto w-full"
        preserveAspectRatio="xMidYMid meet"
        role="img"
        aria-label={`Cumulative position P&L chart ending at ${fmt(lastV)}`}
      >
        <defs>
          <linearGradient id="pnlGradPosTradeHistory" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="#10b981" stopOpacity="0.24" />
            <stop offset="100%" stopColor="#10b981" stopOpacity="0.02" />
          </linearGradient>
          <linearGradient id="pnlGradNegTradeHistory" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="#f43f5e" stopOpacity="0.03" />
            <stop offset="100%" stopColor="#f43f5e" stopOpacity="0.2" />
          </linearGradient>
          <clipPath id="aboveZeroTradeHistory">
            <rect x={PAD.left} y={PAD.top} width={iW} height={Math.max(0, zeroY - PAD.top)} />
          </clipPath>
          <clipPath id="belowZeroTradeHistory">
            <rect x={PAD.left} y={zeroY} width={iW} height={Math.max(0, PAD.top + lineH - zeroY)} />
          </clipPath>
        </defs>

        {yTicks.map((tk, i) => (
          <g key={i}>
            <line x1={PAD.left} x2={W - PAD.right} y1={tk.y} y2={tk.y}
              stroke="#1f2937" strokeDasharray="5 5" strokeWidth={1} />
            <text x={PAD.left - 10} y={tk.y + 4} textAnchor="end" fontSize={11} fill="#6b7280">
              {tk.label}
            </text>
          </g>
        ))}

        <line x1={PAD.left} x2={W - PAD.right} y1={zeroY} y2={zeroY}
          stroke="#64748b" strokeWidth={1.2} />

        <path d={areaPath} fill="url(#pnlGradPosTradeHistory)" clipPath="url(#aboveZeroTradeHistory)" />
        <path d={areaPath} fill="url(#pnlGradNegTradeHistory)" clipPath="url(#belowZeroTradeHistory)" />
        <path d={linePath} fill="none" stroke={lineColor} strokeWidth={2.5}
          strokeLinejoin="round" strokeLinecap="round" />

        <line x1={PAD.left} x2={W - PAD.right} y1={barBase} y2={barBase} stroke="#1f2937" strokeWidth={1} />
        {dailyRows.map((row) => {
          const x = ix(row.t) - barW / 2;
          const h = Math.max(2, (Math.abs(row.amount) / maxDailyAbs) * barMaxH);
          const y = row.amount >= 0 ? barBase - h : barBase;
          return (
            <rect
              key={row.key}
              x={x}
              y={y}
              width={barW}
              height={h}
              rx={2}
              fill={row.amount >= 0 ? "#10b981" : "#f43f5e"}
              opacity={0.78}
            />
          );
        })}
        <text x={PAD.left} y={barTop - 6} fontSize={10} fontWeight={700} fill="#64748b">
          DAILY P&L
        </text>

        <line x1={last.x} x2={last.x} y1={PAD.top} y2={barBase + barMaxH / 2}
          stroke="#334155" strokeDasharray="4 5" strokeWidth={1} />
        <circle cx={last.x} cy={last.y} r={7} fill={lineColor} fillOpacity={0.16} />
        <circle cx={last.x} cy={last.y} r={3.8} fill={lineColor} />
        <text x={Math.min(last.x + 10, W - PAD.right - 78)} y={Math.max(PAD.top + 13, last.y - 8)}
          fontSize={11} fontWeight={700} fill={lineColor}>
          {fmt(lastV)}
        </text>

        {xTicks.map((tk) => (
          <text key={tk.label} x={tk.x} y={H - 6} textAnchor="middle" fontSize={10} fill="#6b7280">
            {tk.label}
          </text>
        ))}
      </svg>

      <div className="grid grid-cols-2 gap-2 border-t border-gray-800 pt-2 sm:grid-cols-3 xl:grid-cols-5">
        {statRows.map((row) => (
          <div key={row.label} className="min-w-0 rounded-md bg-gray-900/45 px-2 py-1.5">
            <div className="text-[9px] font-semibold uppercase tracking-wide text-gray-600">{row.label}</div>
            <div className={`mt-0.5 text-xs font-semibold tabular-nums ${row.color}`}>{row.value}</div>
            {row.sub && <div className="text-[9px] text-gray-600">{row.sub}</div>}
          </div>
        ))}
      </div>
    </div>
  );
}

// ── Orders per day bar chart ──────────────────────────────────────────────────
function OrdersBarChart({ orders, height = 200 }: { orders: any[]; height?: number }) {
  const W = 900;
  const H = height;
  const PAD = { top: 22, right: 20, bottom: 34, left: 44 };

  const buckets = useMemo(() => {
    const map = new Map<string, { count: number; notional: number }>();
    for (const o of orders) {
      if (!o.submitted_at) continue;
      const key = dayKey(o.submitted_at);
      const row = map.get(key) ?? { count: 0, notional: 0 };
      row.count += 1;
      row.notional += Number(o.notional ?? 0);
      map.set(key, row);
    }
    return Array.from(map.entries())
      .sort((a, b) => a[0].localeCompare(b[0]))
      .slice(-30)
      .map(([key, row]) => ({
        label: dayLabel(key),
        count: row.count,
        notional: row.notional,
      }));
  }, [orders]);

  if (buckets.length === 0) {
    return <EmptyChart height={height} label="No order data in range" />;
  }

  const maxCount = Math.max(...buckets.map((b) => b.count));
  const totalOrders = buckets.reduce((sum, b) => sum + b.count, 0);
  const totalNotional = buckets.reduce((sum, b) => sum + b.notional, 0);
  const avg = totalOrders / buckets.length;
  const peak = buckets.reduce((a, b) => (b.count > a.count ? b : a), buckets[0]);
  const innerW = W - PAD.left - PAD.right;
  const innerH = H - PAD.top - PAD.bottom;
  const slot = innerW / buckets.length;
  const barW = Math.max(4, Math.min(22, slot * 0.62));
  const avgY = PAD.top + (1 - avg / maxCount) * innerH;
  const yTicks = Array.from(new Set([0, Math.round(maxCount * 0.5), maxCount]));

  return (
    <div className="space-y-3">
      <div className="grid grid-cols-3 gap-3 border-b border-gray-800 pb-2">
        <div>
          <div className="text-[10px] font-semibold uppercase tracking-wide text-gray-600">Active days</div>
          <div className="mt-1 text-sm font-semibold tabular-nums text-gray-100">{buckets.length}</div>
        </div>
        <div>
          <div className="text-[10px] font-semibold uppercase tracking-wide text-gray-600">Peak day</div>
          <div className="mt-1 text-sm font-semibold tabular-nums text-sky-300">
            {peak.count} <span className="text-[10px] text-gray-600">{peak.label}</span>
          </div>
        </div>
        <div>
          <div className="text-[10px] font-semibold uppercase tracking-wide text-gray-600">Avg notional</div>
          <div className="mt-1 text-sm font-semibold tabular-nums text-gray-100">
            {totalOrders > 0 ? fmtCompact(totalNotional / totalOrders) : "$0"}
          </div>
        </div>
      </div>

      <svg viewBox={`0 0 ${W} ${H}`} className="block w-full" preserveAspectRatio="xMidYMid meet">
        <defs>
          <linearGradient id="ordersBarTradeHistory" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="#38bdf8" stopOpacity="0.96" />
            <stop offset="100%" stopColor="#0f766e" stopOpacity="0.9" />
          </linearGradient>
        </defs>
        {yTicks.map((val, i) => {
          const y = PAD.top + (1 - val / maxCount) * innerH;
          return (
            <g key={i}>
              <line x1={PAD.left} x2={W - PAD.right} y1={y} y2={y}
                stroke="#1f2937" strokeDasharray="5 5" strokeWidth={1} />
              <text x={PAD.left - 6} y={y + 4} textAnchor="end" fontSize={11} fill="#6b7280">
                {val}
              </text>
            </g>
          );
        })}
        <line x1={PAD.left} x2={W - PAD.right} y1={avgY} y2={avgY}
          stroke="#f59e0b" strokeWidth={1.2} strokeDasharray="4 5" opacity={0.9} />
        <text x={W - PAD.right - 4} y={avgY - 7} textAnchor="end" fontSize={10} fill="#fbbf24">
          avg {avg.toFixed(1)}
        </text>

        {buckets.map(({ label, count }, i) => {
          const barH = (count / maxCount) * innerH;
          const x = PAD.left + i * slot + (slot - barW) / 2;
          const y = PAD.top + innerH - barH;
          const showLabel =
            buckets.length <= 14 || i % Math.ceil(buckets.length / 8) === 0 || i === buckets.length - 1;
          return (
            <g key={label}>
              <rect x={x} y={y} width={barW} height={Math.max(2, barH)} rx={4}
                fill="url(#ordersBarTradeHistory)" opacity={0.92} />
              {count > 0 && barH > 20 && (
                <text x={x + barW / 2} y={y + 13} textAnchor="middle" fontSize={10} fill="#ecfeff" opacity={0.86}>
                  {count}
                </text>
              )}
              {showLabel && (
                <text x={x + barW / 2} y={H - 8} textAnchor="middle" fontSize={10} fill="#6b7280">
                  {label}
                </text>
              )}
            </g>
          );
        })}
      </svg>
    </div>
  );
}

// ── Daily P&L chart ───────────────────────────────────────────────────────────
function DailyPnlChart({ positions, height = 200 }: { positions: any[]; height?: number }) {
  const W = 900;
  const H = height;
  const PAD = { top: 18, right: 18, bottom: 34, left: 70 };

  const buckets = useMemo(() => {
    const map = new Map<string, number>();
    for (const p of positions) {
      if (!p.updated_at) continue;
      const key = dayKey(p.updated_at);
      const pnl = positionPnl(p);
      if (!Number.isFinite(pnl)) continue;
      map.set(key, (map.get(key) ?? 0) + pnl);
    }
    return Array.from(map.entries())
      .sort((a, b) => a[0].localeCompare(b[0]))
      .slice(-30)
      .map(([key, total]) => ({
        label: dayLabel(key),
        total,
      }));
  }, [positions]);

  if (buckets.length === 0) {
    return <EmptyChart height={height} label="No position P&L data in range" />;
  }

  const vals = buckets.map((b) => b.total);
  const maxAbs = Math.max(1, ...vals.map(Math.abs));
  const innerW = W - PAD.left - PAD.right;
  const innerH = H - PAD.top - PAD.bottom;
  const slot = innerW / buckets.length;
  const barW = Math.max(4, Math.min(24, slot * 0.62));
  const midY = PAD.top + innerH / 2;
  const wins = buckets.filter((b) => b.total > 0).length;
  const losses = buckets.filter((b) => b.total < 0).length;
  const best = buckets.reduce((a, b) => (b.total > a.total ? b : a), buckets[0]);
  const worst = buckets.reduce((a, b) => (b.total < a.total ? b : a), buckets[0]);

  return (
    <div className="space-y-3">
      <div className="grid grid-cols-3 gap-3 border-b border-gray-800 pb-2">
        <div>
          <div className="text-[10px] font-semibold uppercase tracking-wide text-gray-600">Positive days</div>
          <div className="mt-1 text-sm font-semibold tabular-nums text-emerald-300">{wins}</div>
        </div>
        <div>
          <div className="text-[10px] font-semibold uppercase tracking-wide text-gray-600">Best day</div>
          <div className="mt-1 text-sm font-semibold tabular-nums text-emerald-300">
            {fmtCompact(best.total)} <span className="text-[10px] text-gray-600">{best.label}</span>
          </div>
        </div>
        <div>
          <div className="text-[10px] font-semibold uppercase tracking-wide text-gray-600">Worst day</div>
          <div className="mt-1 text-sm font-semibold tabular-nums text-rose-300">
            {fmtCompact(worst.total)} <span className="text-[10px] text-gray-600">{worst.label}</span>
          </div>
        </div>
      </div>

      <svg viewBox={`0 0 ${W} ${H}`} className="block w-full" preserveAspectRatio="xMidYMid meet">
        <defs>
          <linearGradient id="dailyPnlWinTradeHistory" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="#34d399" stopOpacity="0.95" />
            <stop offset="100%" stopColor="#059669" stopOpacity="0.78" />
          </linearGradient>
          <linearGradient id="dailyPnlLossTradeHistory" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="#fb7185" stopOpacity="0.78" />
            <stop offset="100%" stopColor="#e11d48" stopOpacity="0.96" />
          </linearGradient>
        </defs>
        {[-maxAbs, 0, maxAbs].map((v, i) => {
          const y = midY - (v / maxAbs) * (innerH / 2);
          return (
            <g key={i}>
              <line x1={PAD.left} x2={W - PAD.right} y1={y} y2={y}
                stroke={v === 0 ? "#94a3b8" : "#1f2937"}
                strokeWidth={v === 0 ? 1.5 : 1}
                strokeDasharray={v === 0 ? undefined : "5 5"} />
              <text x={PAD.left - 6} y={y + 4} textAnchor="end" fontSize={11} fill="#6b7280">
                {fmtCompact(v)}
              </text>
            </g>
          );
        })}

        {buckets.map(({ label, total }, i) => {
          const barH = Math.max(2, Math.abs(total / maxAbs) * (innerH / 2));
          const x = PAD.left + i * slot + (slot - barW) / 2;
          const y = total >= 0 ? midY - barH : midY;
          const fill = total >= 0 ? "url(#dailyPnlWinTradeHistory)" : "url(#dailyPnlLossTradeHistory)";
          const showLabel =
            buckets.length <= 14 || i % Math.ceil(buckets.length / 8) === 0 || i === buckets.length - 1;
          return (
            <g key={label}>
              <rect x={x} y={y} width={barW} height={barH} rx={4} fill={fill} opacity={0.92} />
              {Math.abs(total) > 0 && barH > 28 && (
                <text
                  x={x + barW / 2}
                  y={total >= 0 ? y + 13 : y + barH - 5}
                  textAnchor="middle"
                  fontSize={9}
                  fill="#f8fafc"
                  opacity={0.85}
                >
                  {fmtCompact(total)}
                </text>
              )}
              {showLabel && (
                <text x={x + barW / 2} y={H - 8} textAnchor="middle" fontSize={10} fill="#6b7280">
                  {label}
                </text>
              )}
            </g>
          );
        })}
        <text x={W - PAD.right} y={midY - 7} textAnchor="end" fontSize={10} fill="#94a3b8">
          zero
        </text>
      </svg>
      <div className="text-[10px] text-gray-600">
        {losses} loss day{losses === 1 ? "" : "s"} across {buckets.length} active P&L day{buckets.length === 1 ? "" : "s"}.
      </div>
    </div>
  );
}

function OrderStatusChart({ orders }: { orders: any[] }) {
  const rows = useMemo(() => {
    const map = new Map<string, number>();
    for (const order of orders) {
      const key = String(order.status ?? "unknown");
      map.set(key, (map.get(key) ?? 0) + 1);
    }
    return Array.from(map.entries())
      .sort((a, b) => b[1] - a[1])
      .map(([status, count]) => ({ status, count }));
  }, [orders]);

  if (rows.length === 0) return <EmptyChart height={150} label="No order status data" />;

  const max = Math.max(...rows.map((r) => r.count));
  const color = (status: string) => {
    if (status === "filled" || status === "executed") return "bg-emerald-400";
    if (status === "pending" || status === "approved") return "bg-amber-400";
    if (status === "cancelled" || status === "rejected" || status === "failed") return "bg-rose-400";
    return "bg-slate-400";
  };

  return (
    <div className="space-y-3">
      {rows.map((row) => (
        <div key={row.status} className="grid grid-cols-[88px_1fr_48px] items-center gap-3">
          <span className="truncate text-xs capitalize text-gray-400">{row.status}</span>
          <div className="h-3 overflow-hidden rounded-full bg-gray-900">
            <div
              className={`h-full rounded-full ${color(row.status)}`}
              style={{ width: `${Math.max(6, (row.count / max) * 100)}%` }}
            />
          </div>
          <span className="text-right text-xs font-semibold tabular-nums text-gray-200">{row.count}</span>
        </div>
      ))}
    </div>
  );
}

function PlatformExposureChart({ positions }: { positions: any[] }) {
  const rows = useMemo(() => {
    const map = new Map<string, number>();
    for (const position of positions) {
      const key = String(position.platform ?? "unknown");
      map.set(key, (map.get(key) ?? 0) + Number(position.exposure ?? 0));
    }
    return Array.from(map.entries())
      .sort((a, b) => b[1] - a[1])
      .map(([platform, exposure]) => ({ platform, exposure }));
  }, [positions]);

  if (rows.length === 0) return <EmptyChart height={150} label="No exposure data" />;

  const total = rows.reduce((s, r) => s + r.exposure, 0);
  const max = Math.max(...rows.map((r) => r.exposure));

  return (
    <div className="space-y-3">
      {rows.map((row) => {
        const pct = total > 0 ? (row.exposure / total) * 100 : 0;
        const bar = max > 0 ? (row.exposure / max) * 100 : 0;
        const color = row.platform === "kalshi" ? "bg-sky-400" : "bg-violet-400";
        return (
          <div key={row.platform} className="grid grid-cols-[92px_1fr_84px] items-center gap-3">
            <span className="truncate text-xs capitalize text-gray-400">{row.platform}</span>
            <div className="h-3 overflow-hidden rounded-full bg-gray-900">
              <div className={`h-full rounded-full ${color}`} style={{ width: `${Math.max(6, bar)}%` }} />
            </div>
            <span className="text-right text-xs font-semibold tabular-nums text-gray-200">
              {fmtCompact(row.exposure)} <span className="text-gray-600">{pct.toFixed(0)}%</span>
            </span>
          </div>
        );
      })}
    </div>
  );
}

function StatusExposureChart({ orders, positions }: { orders: any[]; positions: any[] }) {
  return (
    <div className="space-y-5">
      <div>
        <div className="mb-2 flex items-center justify-between gap-3">
          <div className="text-[10px] font-semibold uppercase tracking-wide text-gray-600">
            Order status
          </div>
          <div className="text-[10px] text-gray-600">{orders.length} orders</div>
        </div>
        <OrderStatusChart orders={orders} />
      </div>

      <div className="border-t border-gray-800 pt-4">
        <div className="mb-2 flex items-center justify-between gap-3">
          <div className="text-[10px] font-semibold uppercase tracking-wide text-gray-600">
            Exposure by platform
          </div>
          <div className="text-[10px] text-gray-600">
            {fmtCompact(positions.reduce((sum, position) => sum + Number(position.exposure ?? 0), 0))}
          </div>
        </div>
        <PlatformExposureChart positions={positions} />
      </div>
    </div>
  );
}

// ── Tab tables ────────────────────────────────────────────────────────────────
function OrdersTab({ data }: { data: any[] }) {
  const [page, setPage] = useState(1);
  const rows = data.slice((page - 1) * PER_PAGE, page * PER_PAGE);
  const totalPages = Math.max(1, Math.ceil(data.length / PER_PAGE));

  return (
    <>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-gray-800 text-left text-[11px] font-semibold uppercase tracking-wider text-gray-500">
              <th className="px-4 py-3">Platform</th>
              <th className="px-4 py-3">Market</th>
              <th className="px-4 py-3">Side</th>
              <th className="px-4 py-3 text-right">Qty</th>
              <th className="px-4 py-3 text-right">Price</th>
              <th className="px-4 py-3 text-right">Notional</th>
              <th className="px-4 py-3">Status</th>
              <th className="px-4 py-3 text-right">Created</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-800/60">
            {rows.map((o: any) => (
              <tr key={o.id} className="hover:bg-gray-900/50 transition-colors">
                <td className="px-4 py-3">
                  <span className={`rounded-md px-2 py-0.5 text-xs font-medium ${
                    o.platform === "kalshi"
                      ? "bg-blue-900/40 text-blue-300"
                      : "bg-purple-900/40 text-purple-300"
                  }`}>
                    {o.platform ?? "—"}
                  </span>
                </td>
                <td className="px-4 py-3 max-w-[200px]">
                  <span
                    className="block truncate text-xs text-gray-300 font-mono"
                    title={o.market_id}
                  >
                    {o.market_id ? o.market_id.slice(0, 20) + "…" : "—"}
                  </span>
                </td>
                <td className="px-4 py-3">
                  <span className={`rounded-md px-2 py-0.5 text-xs font-bold ${
                    o.side === "buy"
                      ? "bg-green-900/50 text-green-300"
                      : "bg-red-900/50 text-red-300"
                  }`}>
                    {o.side?.toUpperCase() ?? "—"}
                  </span>
                </td>
                <td className="px-4 py-3 text-right text-xs text-gray-300 tabular-nums">
                  {o.requested_contracts != null ? Number(o.requested_contracts).toFixed(2) : "—"}
                </td>
                <td className="px-4 py-3 text-right text-xs text-gray-300 tabular-nums">
                  {o.price_limit != null ? `${(Number(o.price_limit) * 100).toFixed(0)}¢` : "—"}
                </td>
                <td className="px-4 py-3 text-right text-xs text-gray-300 tabular-nums">
                  {o.notional != null ? fmt(Number(o.notional)) : "—"}
                </td>
                <td className="px-4 py-3">
                  <span className={`rounded-md px-2 py-0.5 text-xs font-medium ${
                    o.status === "filled"
                      ? "bg-green-900/50 text-green-300"
                      : o.status === "pending"
                      ? "bg-yellow-900/50 text-yellow-300"
                      : o.status === "cancelled"
                      ? "bg-red-900/50 text-red-300"
                      : "bg-gray-800 text-gray-400"
                  }`}>
                    {o.status ?? "—"}
                  </span>
                </td>
                <td className="px-4 py-3 text-right text-xs text-gray-500 whitespace-nowrap">
                  {o.submitted_at
                    ? new Date(o.submitted_at).toLocaleString("en-US", {
                        month: "short", day: "numeric",
                        hour: "2-digit", minute: "2-digit",
                      })
                    : "—"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {rows.length === 0 && (
        <p className="px-4 py-10 text-center text-sm text-gray-600">No orders found.</p>
      )}
      <Pagination page={page} totalPages={totalPages} totalItems={data.length} perPage={PER_PAGE} onChange={setPage} />
    </>
  );
}

function PositionsTab({ data }: { data: any[] }) {
  const [page, setPage] = useState(1);
  const rows = data.slice((page - 1) * PER_PAGE, page * PER_PAGE);
  const totalPages = Math.max(1, Math.ceil(data.length / PER_PAGE));

  return (
    <>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-gray-800 text-left text-[11px] font-semibold uppercase tracking-wider text-gray-500">
              <th className="px-4 py-3">Market</th>
              <th className="px-4 py-3">Platform</th>
              <th className="px-4 py-3">Outcome</th>
              <th className="px-4 py-3 text-right">Contracts</th>
              <th className="px-4 py-3 text-right">Avg Price</th>
              <th className="px-4 py-3 text-right">Exposure</th>
              <th className="px-4 py-3 text-right">Realized P&L</th>
              <th className="px-4 py-3 text-right">Updated</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-800/60">
            {rows.map((p: any) => (
              <tr key={p.id} className="hover:bg-gray-900/50 transition-colors">
                <td className="px-4 py-3 max-w-[200px]">
                  <span className="block truncate text-xs text-gray-200" title={p.market_title || p.market_id}>
                    {p.market_title || (p.market_id ? p.market_id.slice(0, 18) + "…" : "—")}
                  </span>
                </td>
                <td className="px-4 py-3">
                  <span className={`rounded-md px-2 py-0.5 text-xs font-medium ${
                    p.platform === "kalshi"
                      ? "bg-blue-900/40 text-blue-300"
                      : "bg-purple-900/40 text-purple-300"
                  }`}>
                    {p.platform}
                  </span>
                </td>
                <td className="px-4 py-3 text-xs text-gray-300 capitalize">{p.outcome ?? "—"}</td>
                <td className="px-4 py-3 text-right text-xs text-gray-300 tabular-nums">
                  {Number(p.contracts).toFixed(2)}
                </td>
                <td className="px-4 py-3 text-right text-xs text-gray-300 tabular-nums">
                  {(Number(p.avg_price) * 100).toFixed(0)}¢
                </td>
                <td className="px-4 py-3 text-right text-xs text-gray-300 tabular-nums">
                  {fmt(Number(p.exposure))}
                </td>
                <td className={`px-4 py-3 text-right text-xs font-semibold tabular-nums ${
                  Number(p.realized_pnl) > 0
                    ? "text-green-400"
                    : Number(p.realized_pnl) < 0
                    ? "text-red-400"
                    : "text-gray-500"
                }`}>
                  {fmt(Number(p.realized_pnl))}
                </td>
                <td className="px-4 py-3 text-right text-xs text-gray-500 whitespace-nowrap">
                  {new Date(p.updated_at).toLocaleString("en-US", {
                    month: "short", day: "numeric",
                    hour: "2-digit", minute: "2-digit",
                  })}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {rows.length === 0 && (
        <p className="px-4 py-10 text-center text-sm text-gray-600">No positions found.</p>
      )}
      <Pagination page={page} totalPages={totalPages} totalItems={data.length} perPage={PER_PAGE} onChange={setPage} />
    </>
  );
}

// ── Calendar popup ────────────────────────────────────────────────────────────
const DOW = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];

function toIso(y: number, m: number, d: number) {
  return `${y}-${String(m + 1).padStart(2, "0")}-${String(d).padStart(2, "0")}`;
}
function fmtDisplay(iso: string) {
  const [y, m, d] = iso.split("-");
  return `${d}/${m}/${y}`;
}

function CalendarPopup({
  value, minDate, maxDate, highlightFrom, highlightTo, onSelect, onClose,
}: {
  value: string;
  minDate?: string;
  maxDate?: string;
  highlightFrom?: string;
  highlightTo?: string;
  onSelect: (iso: string) => void;
  onClose: () => void;
}) {
  const todayIso = toIso(new Date().getFullYear(), new Date().getMonth(), new Date().getDate());
  const init = value || todayIso;
  const [viewYear, setViewYear] = useState(Number(init.slice(0, 4)));
  const [viewMonth, setViewMonth] = useState(Number(init.slice(5, 7)) - 1);

  const daysInMonth = new Date(viewYear, viewMonth + 1, 0).getDate();
  const firstDow = new Date(viewYear, viewMonth, 1).getDay();
  const monthLabel = new Date(viewYear, viewMonth, 1).toLocaleDateString("en-US", {
    month: "long", year: "numeric",
  });

  function prevMonth() {
    if (viewMonth === 0) { setViewYear((y) => y - 1); setViewMonth(11); }
    else setViewMonth((m) => m - 1);
  }
  function nextMonth() {
    if (viewMonth === 11) { setViewYear((y) => y + 1); setViewMonth(0); }
    else setViewMonth((m) => m + 1);
  }

  const cells: (number | null)[] = [
    ...Array(firstDow).fill(null),
    ...Array.from({ length: daysInMonth }, (_, i) => i + 1),
  ];

  return (
    <div className="absolute z-50 top-full mt-1 rounded-lg border border-gray-700 bg-gray-900 shadow-2xl p-3 w-60 select-none">
      <div className="flex items-center justify-between mb-2">
        <button onClick={prevMonth} className="p-1 rounded hover:bg-gray-800 text-gray-400 hover:text-white transition-colors">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
            <path d="M15 18l-6-6 6-6" />
          </svg>
        </button>
        <span className="text-xs font-semibold text-gray-200">{monthLabel}</span>
        <button onClick={nextMonth} className="p-1 rounded hover:bg-gray-800 text-gray-400 hover:text-white transition-colors">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
            <path d="M9 18l6-6-6-6" />
          </svg>
        </button>
      </div>
      <div className="grid grid-cols-7 mb-0.5">
        {DOW.map((d) => (
          <div key={d} className="text-center text-[10px] text-gray-600 py-0.5 font-medium">{d}</div>
        ))}
      </div>
      <div className="grid grid-cols-7 gap-y-0.5">
        {cells.map((day, i) => {
          if (day === null) return <div key={`e${i}`} />;
          const iso = toIso(viewYear, viewMonth, day);
          const disabled = (minDate && iso < minDate) || (maxDate && iso > maxDate);
          const isSelected = iso === value;
          const isStart = iso === highlightFrom;
          const isEnd = iso === highlightTo;
          const inRng = highlightFrom && highlightTo && iso > highlightFrom && iso < highlightTo;
          const isToday = iso === todayIso;
          return (
            <button
              key={iso}
              disabled={!!disabled}
              onClick={() => onSelect(iso)}
              className={[
                "text-[11px] py-1 rounded text-center transition-colors leading-none",
                disabled ? "text-gray-700 cursor-not-allowed" : "cursor-pointer",
                isSelected || isStart || isEnd
                  ? "bg-blue-600 text-white font-bold"
                  : inRng
                  ? "bg-blue-900/40 text-blue-300"
                  : isToday
                  ? "text-blue-400 font-semibold hover:bg-gray-800"
                  : !disabled
                  ? "text-gray-300 hover:bg-gray-800"
                  : "",
              ].filter(Boolean).join(" ")}
            >
              {day}
            </button>
          );
        })}
      </div>
      <div className="mt-2 pt-2 border-t border-gray-800 flex justify-between">
        {value && (
          <button
            onClick={() => { onSelect(""); onClose(); }}
            className="text-[10px] text-gray-500 hover:text-gray-300 transition-colors"
          >
            Clear date
          </button>
        )}
        <button onClick={onClose} className="text-[10px] text-gray-500 hover:text-gray-300 transition-colors ml-auto">
          Done
        </button>
      </div>
    </div>
  );
}

// ── Date range picker ─────────────────────────────────────────────────────────
function DateRangePicker({
  from, to, onChange, onClear,
}: {
  from: string; to: string;
  onChange: (from: string, to: string) => void;
  onClear: () => void;
}) {
  const [open, setOpen] = useState<"from" | "to" | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handler(e: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(null);
      }
    }
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  return (
    <div ref={containerRef} className="flex items-center gap-2 flex-wrap">
      <span className="text-xs text-gray-500 shrink-0">Date range:</span>
      <div className="relative">
        <button
          onClick={() => setOpen(open === "from" ? null : "from")}
          className={`rounded-lg border px-3 py-1.5 text-xs transition-colors ${
            from
              ? "border-blue-600 bg-blue-600/10 text-blue-300"
              : "border-gray-700 bg-gray-900 text-gray-400 hover:border-gray-500 hover:text-gray-200"
          }`}
        >
          {from ? fmtDisplay(from) : "From"}
        </button>
        {open === "from" && (
          <CalendarPopup
            value={from} maxDate={to || undefined}
            highlightFrom={from} highlightTo={to}
            onSelect={(d) => { onChange(d, to); if (d) setOpen("to"); else setOpen(null); }}
            onClose={() => setOpen(null)}
          />
        )}
      </div>
      <span className="text-gray-700 text-xs">→</span>
      <div className="relative">
        <button
          onClick={() => setOpen(open === "to" ? null : "to")}
          className={`rounded-lg border px-3 py-1.5 text-xs transition-colors ${
            to
              ? "border-blue-600 bg-blue-600/10 text-blue-300"
              : "border-gray-700 bg-gray-900 text-gray-400 hover:border-gray-500 hover:text-gray-200"
          }`}
        >
          {to ? fmtDisplay(to) : "To"}
        </button>
        {open === "to" && (
          <CalendarPopup
            value={to} minDate={from || undefined}
            highlightFrom={from} highlightTo={to}
            onSelect={(d) => { onChange(from, d); setOpen(null); }}
            onClose={() => setOpen(null)}
          />
        )}
      </div>
      {(from || to) && (
        <button
          onClick={onClear}
          className="rounded-lg px-2.5 py-1.5 text-xs text-gray-500 hover:text-white hover:bg-gray-800 transition-colors"
        >
          Clear
        </button>
      )}
    </div>
  );
}

// ── Helpers ───────────────────────────────────────────────────────────────────
const TABS: { id: Tab; label: string }[] = [
  { id: "orders", label: "Orders" },
  { id: "positions", label: "Positions" },
];

function inRange(dateStr: string | null | undefined, from: string, to: string): boolean {
  if (!dateStr) return true;
  const t = new Date(dateStr).getTime();
  if (from && t < new Date(from).getTime()) return false;
  if (to && t > new Date(to + "T23:59:59").getTime()) return false;
  return true;
}

// ── Page ──────────────────────────────────────────────────────────────────────
export function TradeHistoryPage() {
  const [activeTab, setActiveTab] = useState<Tab>("orders");
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");

  const { data: orders } = useOrders(HISTORY_LIMIT);
  const { data: positions } = usePositions(HISTORY_LIMIT);
  const { data: walletMovements } = usePaperWalletMovements(HISTORY_LIMIT);
  const { data: pnlSummaries } = usePnlSummary();

  const allOrders = useMemo(
    () => [...(orders ?? [])].sort((a, b) =>
      new Date(b.submitted_at ?? 0).getTime() - new Date(a.submitted_at ?? 0).getTime()
    ),
    [orders]
  );
  const allPositions = useMemo(
    () => [...(positions ?? [])].sort((a, b) =>
      new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
    ),
    [positions]
  );
  const allMovements = useMemo(
    () => [...(walletMovements ?? [])].sort((a, b) =>
      new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
    ),
    [walletMovements]
  );

  const filteredOrders = useMemo(
    () => allOrders.filter((o) => inRange(o.submitted_at, dateFrom, dateTo)),
    [allOrders, dateFrom, dateTo]
  );
  const filteredPositions = useMemo(
    () => allPositions.filter((p) => inRange(p.updated_at, dateFrom, dateTo)),
    [allPositions, dateFrom, dateTo]
  );
  const filteredMovements = useMemo(
    () => allMovements.filter((m) => inRange(m.created_at, dateFrom, dateTo)),
    [allMovements, dateFrom, dateTo]
  );

  const pnlSummary = useMemo(() => {
    const rows = pnlSummaries ?? [];
    return rows.find((row) => row.follower_account_id === PAPER_FOLLOWER_ID) ?? rows[0] ?? null;
  }, [pnlSummaries]);

  // Stats
  const filledOrders = filteredOrders.filter((o) => o.status === "filled").length;
  const openPositions = filteredPositions.filter((p) => Number(p.contracts) > 0).length;
  const filteredRealizedPnl = filteredPositions.reduce((s, p) => s + Number(p.realized_pnl ?? 0), 0);
  const filteredUnrealizedPnl = filteredPositions.reduce((s, p) => s + Number(p.unrealized_pnl ?? 0), 0);
  const filteredPositionPnl = filteredRealizedPnl + filteredUnrealizedPnl;
  const officialRealizedPnl = Number(pnlSummary?.realized_pnl ?? filteredRealizedPnl);
  const officialUnrealizedPnl = Number(pnlSummary?.unrealized_pnl ?? filteredUnrealizedPnl);
  const officialTotalPnl = Number(pnlSummary?.total_pnl ?? filteredPositionPnl);
  const hasDateFilter = Boolean(dateFrom || dateTo);
  const visiblePnl = hasDateFilter ? filteredPositionPnl : officialTotalPnl;
  const visibleRealizedPnl = hasDateFilter ? filteredRealizedPnl : officialRealizedPnl;
  const visibleUnrealizedPnl = hasDateFilter ? filteredUnrealizedPnl : officialUnrealizedPnl;
  const walletCashFlow = filteredMovements.reduce((s, m) => s + Number(m.amount), 0);
  const totalExposure = filteredPositions.reduce((s, p) => s + Number(p.exposure ?? 0), 0);
  const filledRate = filteredOrders.length > 0 ? (filledOrders / filteredOrders.length) * 100 : 0;
  const avgNotional = filteredOrders.length > 0
    ? filteredOrders.reduce((s, o) => s + Number(o.notional ?? 0), 0) / filteredOrders.length
    : 0;
  const lastMovement = filteredMovements.length > 0
    ? filteredMovements[filteredMovements.length - 1]
    : null;
  const isPossiblyLimited =
    allOrders.length >= HISTORY_LIMIT ||
    allPositions.length >= HISTORY_LIMIT ||
    allMovements.length >= HISTORY_LIMIT;

  const counts: Record<Tab, number> = {
    orders: filteredOrders.length,
    positions: filteredPositions.length,
  };

  return (
    <div className="space-y-4">
      <div className="rounded-lg border border-gray-800 bg-gray-950 p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 className="text-xl font-semibold text-gray-100">Trade History</h2>
            <p className="mt-1 text-xs text-gray-500">
              Paper trading performance, exposure, and execution quality
            </p>
          </div>
          <DateRangePicker
            from={dateFrom} to={dateTo}
            onChange={(f, t) => { setDateFrom(f); setDateTo(t); }}
            onClear={() => { setDateFrom(""); setDateTo(""); }}
          />
        </div>
        <div className="mt-4 grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
          <StatCard
            label="Total Orders"
            value={filteredOrders.length.toLocaleString()}
            sub={`${filledOrders} filled • ${filledRate.toFixed(0)}% fill rate`}
            accent="bg-sky-400"
          />
          <StatCard
            label="Open Positions"
            value={openPositions.toLocaleString()}
            sub={`${filteredPositions.length} tracked • ${fmtCompact(totalExposure)} exposure`}
            accent="bg-violet-400"
          />
          <StatCard
            label={hasDateFilter ? "Selected P&L" : "Total P&L"}
            value={fmt(visiblePnl)}
            sub={`${fmt(visibleRealizedPnl)} realized • ${fmt(visibleUnrealizedPnl)} unrealized`}
            valueColor={moneyColor(visiblePnl)}
            accent={visiblePnl >= 0 ? "bg-emerald-400" : "bg-rose-400"}
          />
          <StatCard
            label="Wallet Cash Flow"
            value={fmt(walletCashFlow)}
            sub={lastMovement ? `${filteredMovements.length} events • last ${fmt(Number(lastMovement.amount))}` : "cash ledger events only"}
            valueColor={moneyColor(walletCashFlow)}
            accent="bg-amber-400"
          />
          <StatCard
            label="Avg Order Size"
            value={avgNotional > 0 ? fmt(avgNotional) : "—"}
            sub="notional per order"
            accent="bg-teal-400"
          />
        </div>
        {isPossiblyLimited && (
          <div className="mt-3 rounded-lg border border-amber-800/50 bg-amber-950/20 px-3 py-2 text-xs text-amber-200">
            Showing up to {HISTORY_LIMIT.toLocaleString()} latest rows per dataset. Increase backend pagination before treating older history as complete.
          </div>
        )}
      </div>

      <div className="grid grid-cols-1 gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(280px,0.52fr)]">
        <ChartPanel
          title="Cumulative P&L"
          subtitle="Running realized + unrealized P&L from tracked positions"
          metric={fmt(filteredPositionPnl)}
          metricClass={moneyColor(filteredPositionPnl)}
        >
          <CumulativePnlChart positions={filteredPositions} />
        </ChartPanel>

        <ChartPanel
          title="Status Mix"
          subtitle="Order state and exposure split"
          metric={`${filledOrders} filled`}
        >
          <StatusExposureChart orders={filteredOrders} positions={filteredPositions} />
        </ChartPanel>
      </div>

      <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
        <ChartPanel
          title="Orders Per Day"
          subtitle="Daily execution count across the selected range"
          metric={`${filteredOrders.length} orders`}
        >
          <OrdersBarChart orders={filteredOrders} height={230} />
        </ChartPanel>

        <ChartPanel
          title="Daily P&L"
          subtitle="Position P&L grouped by update date"
          metric={fmt(filteredPositionPnl)}
          metricClass={moneyColor(filteredPositionPnl)}
        >
          <DailyPnlChart positions={filteredPositions} height={230} />
        </ChartPanel>
      </div>

      {/* ── Table ── */}
      <div className="overflow-hidden rounded-lg border border-gray-800 bg-gray-950">
        <div className="flex border-b border-gray-800 bg-gray-950">
          {TABS.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`flex items-center gap-2 px-5 py-3.5 text-sm font-medium transition-colors ${
                activeTab === tab.id
                  ? "border-b-2 border-sky-400 bg-gray-900/40 text-gray-100"
                  : "text-gray-500 hover:text-gray-300"
              }`}
            >
              {tab.label}
              {counts[tab.id] > 0 && (
                <span className="rounded-full bg-gray-800 px-2 py-0.5 text-xs tabular-nums text-gray-400">
                  {counts[tab.id]}
                </span>
              )}
            </button>
          ))}
        </div>

        {activeTab === "orders" && <OrdersTab data={filteredOrders} />}
        {activeTab === "positions" && <PositionsTab data={filteredPositions} />}
      </div>
    </div>
  );
}
