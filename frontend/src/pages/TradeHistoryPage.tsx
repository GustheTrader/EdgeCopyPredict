import { useState, useMemo } from "react";
import { useOrders, usePositions } from "@/hooks/useApi";
import { Pagination } from "@/components/ui/Pagination";

const PER_PAGE = 20;
type Tab = "orders" | "positions";

function fmt(n: number) {
  return `$${n.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
}

// ── SVG area chart: cumulative P&L over time ─────────────────────────────────
function PnlAreaChart({ positions }: { positions: any[] }) {
  const W = 900;
  const H = 160;
  const PAD = { top: 16, right: 16, bottom: 28, left: 56 };

  const points = useMemo(() => {
    const sorted = [...positions]
      .filter((p) => p.updated_at && p.realized_pnl != null)
      .sort((a, b) => new Date(a.updated_at).getTime() - new Date(b.updated_at).getTime());
    let cum = 0;
    return sorted.map((p) => {
      cum += Number(p.realized_pnl) + Number(p.unrealized_pnl ?? 0);
      return { t: new Date(p.updated_at).getTime(), v: cum };
    });
  }, [positions]);

  if (points.length < 2) {
    return (
      <div className="flex h-[160px] items-center justify-center text-sm text-gray-600">
        Not enough data to plot P&L
      </div>
    );
  }

  const minT = points[0].t;
  const maxT = points[points.length - 1].t;
  const values = points.map((p) => p.v);
  const minV = Math.min(0, ...values);
  const maxV = Math.max(0, ...values);
  const rangeV = maxV - minV || 1;

  const ix = (W: number, PAD: any, t: number) =>
    PAD.left + ((t - minT) / (maxT - minT)) * (W - PAD.left - PAD.right);
  const iy = (H: number, PAD: any, v: number) =>
    PAD.top + (1 - (v - minV) / rangeV) * (H - PAD.top - PAD.bottom);

  const coords = points.map((p) => ({ x: ix(W, PAD, p.t), y: iy(H, PAD, p.v) }));
  const polyline = coords.map((c) => `${c.x},${c.y}`).join(" ");
  const zeroY = iy(H, PAD, 0);

  const areaPath =
    `M ${coords[0].x},${zeroY} ` +
    coords.map((c) => `L ${c.x},${c.y}`).join(" ") +
    ` L ${coords[coords.length - 1].x},${zeroY} Z`;

  const lastV = values[values.length - 1];
  const color = lastV >= 0 ? "#22c55e" : "#ef4444";
  const areaColor = lastV >= 0 ? "#22c55e22" : "#ef444422";

  // Y-axis labels
  const yTicks = [minV, minV + rangeV * 0.5, maxV].map((v) => ({
    y: iy(H, PAD, v),
    label: `$${v.toFixed(0)}`,
  }));

  // X-axis labels (up to 5)
  const step = Math.max(1, Math.floor(points.length / 5));
  const xTicks = points.filter((_, i) => i % step === 0 || i === points.length - 1).map((p) => ({
    x: ix(W, PAD, p.t),
    label: new Date(p.t).toLocaleDateString("en-US", { month: "short", day: "numeric" }),
  }));

  return (
    <svg
      viewBox={`0 0 ${W} ${H}`}
      className="w-full"
      style={{ height: H }}
      preserveAspectRatio="none"
    >
      {/* grid lines */}
      {yTicks.map((tk, i) => (
        <line key={i} x1={PAD.left} x2={W - PAD.right} y1={tk.y} y2={tk.y}
          stroke="#1f2937" strokeDasharray="4 4" strokeWidth={1} />
      ))}
      {/* zero line */}
      <line x1={PAD.left} x2={W - PAD.right} y1={zeroY} y2={zeroY}
        stroke="#374151" strokeWidth={1} />

      {/* area fill */}
      <path d={areaPath} fill={areaColor} />

      {/* line */}
      <polyline points={polyline} fill="none" stroke={color} strokeWidth={2} strokeLinejoin="round" />

      {/* dots at each point (only if few points) */}
      {points.length <= 40 && coords.map((c, i) => (
        <circle key={i} cx={c.x} cy={c.y} r={3} fill={color} />
      ))}

      {/* Y labels */}
      {yTicks.map((tk, i) => (
        <text key={i} x={PAD.left - 6} y={tk.y + 4} textAnchor="end"
          fontSize={10} fill="#6b7280">
          {tk.label}
        </text>
      ))}

      {/* X labels */}
      {xTicks.map((tk, i) => (
        <text key={i} x={tk.x} y={H - 4} textAnchor="middle"
          fontSize={10} fill="#6b7280">
          {tk.label}
        </text>
      ))}
    </svg>
  );
}

// ── SVG bar chart: orders per day ────────────────────────────────────────────
function OrdersBarChart({ orders }: { orders: any[] }) {
  const W = 900;
  const H = 160;
  const PAD = { top: 16, right: 16, bottom: 28, left: 40 };

  const buckets = useMemo(() => {
    const map = new Map<string, number>();
    for (const o of orders) {
      const day = new Date(o.submitted_at).toLocaleDateString("en-US", {
        month: "short", day: "numeric",
      });
      map.set(day, (map.get(day) ?? 0) + 1);
    }
    return Array.from(map.entries())
      .sort((a, b) => new Date(a[0]).getTime() - new Date(b[0]).getTime())
      .slice(-30);
  }, [orders]);

  if (buckets.length === 0) {
    return (
      <div className="flex h-[160px] items-center justify-center text-sm text-gray-600">
        No order data to plot
      </div>
    );
  }

  const maxCount = Math.max(...buckets.map((b) => b[1]));
  const innerW = W - PAD.left - PAD.right;
  const innerH = H - PAD.top - PAD.bottom;
  const barW = Math.max(4, innerW / buckets.length - 3);

  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="w-full" style={{ height: H }} preserveAspectRatio="none">
      {[0, 0.5, 1].map((frac, i) => {
        const y = PAD.top + (1 - frac) * innerH;
        return (
          <g key={i}>
            <line x1={PAD.left} x2={W - PAD.right} y1={y} y2={y}
              stroke="#1f2937" strokeDasharray="4 4" strokeWidth={1} />
            <text x={PAD.left - 4} y={y + 4} textAnchor="end" fontSize={10} fill="#6b7280">
              {Math.round(frac * maxCount)}
            </text>
          </g>
        );
      })}
      {buckets.map(([day, count], i) => {
        const barH = (count / maxCount) * innerH;
        const x = PAD.left + (i / buckets.length) * innerW + (innerW / buckets.length - barW) / 2;
        const y = PAD.top + innerH - barH;
        const showLabel = buckets.length <= 20 || i % Math.ceil(buckets.length / 10) === 0;
        return (
          <g key={day}>
            <rect x={x} y={y} width={barW} height={barH} rx={2} fill="#3b82f6" opacity={0.8} />
            {showLabel && (
              <text x={x + barW / 2} y={H - 4} textAnchor="middle" fontSize={9} fill="#6b7280">
                {day}
              </text>
            )}
          </g>
        );
      })}
    </svg>
  );
}

// ── Charts panel ─────────────────────────────────────────────────────────────
function ChartsPanel({ positions, orders }: { positions: any[]; orders: any[] }) {
  const totalPnl = positions.reduce((s, p) => s + Number(p.realized_pnl ?? 0) + Number(p.unrealized_pnl ?? 0), 0);

  return (
    <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
      <div className="rounded-lg border border-gray-800 bg-gray-950 p-4">
        <div className="mb-2 flex items-baseline justify-between">
          <span className="text-xs font-medium uppercase tracking-wide text-gray-500">Cumulative P&L</span>
          <span className={`text-sm font-semibold ${totalPnl >= 0 ? "text-green-400" : "text-red-400"}`}>
            {fmt(totalPnl)}
          </span>
        </div>
        <PnlAreaChart positions={positions} />
      </div>

      <div className="rounded-lg border border-gray-800 bg-gray-950 p-4">
        <div className="mb-2 flex items-baseline justify-between">
          <span className="text-xs font-medium uppercase tracking-wide text-gray-500">Orders Per Day</span>
          <span className="text-sm font-semibold text-gray-300">{orders.length} total</span>
        </div>
        <OrdersBarChart orders={orders} />
      </div>
    </div>
  );
}

// ── Tab tables ────────────────────────────────────────────────────────────────
function SignalsTab({ data }: { data: any[] }) {
  const [page, setPage] = useState(1);
  const rows = data.slice((page - 1) * PER_PAGE, page * PER_PAGE);
  const totalPages = Math.max(1, Math.ceil(data.length / PER_PAGE));

  return (
    <>
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-gray-800 text-left text-gray-400 text-xs uppercase">
            <th className="px-4 py-2">Market</th>
            <th className="px-4 py-2">Platform</th>
            <th className="px-4 py-2">Side</th>
            <th className="px-4 py-2 text-right">Contracts</th>
            <th className="px-4 py-2 text-right">Price</th>
            <th className="px-4 py-2 text-right">Notional</th>
            <th className="px-4 py-2">Status</th>
            <th className="px-4 py-2">Detected</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((s: any) => (
            <tr key={s.id} className="border-b border-gray-800 hover:bg-gray-900">
              <td className="px-4 py-2 max-w-[200px]">
                <span className="truncate block text-gray-200 text-xs" title={s.market_title}>
                  {s.market_title || s.market_id?.slice(0, 10) + "…" || "—"}
                </span>
              </td>
              <td className="px-4 py-2 capitalize text-gray-400 text-xs">{s.platform}</td>
              <td className="px-4 py-2">
                <span className={`rounded px-1.5 py-0.5 text-xs font-medium ${
                  s.side === "buy" ? "bg-green-900/60 text-green-300" : "bg-red-900/60 text-red-300"
                }`}>
                  {s.side?.toUpperCase()}
                </span>
              </td>
              <td className="px-4 py-2 text-right text-gray-300 text-xs">{s.size_contracts?.toFixed(2)}</td>
              <td className="px-4 py-2 text-right text-gray-300 text-xs">
                {s.entry_price != null ? `${(s.entry_price * 100).toFixed(0)}¢` : "—"}
              </td>
              <td className="px-4 py-2 text-right text-gray-300 text-xs">
                {s.notional != null ? `$${Number(s.notional).toFixed(2)}` : "—"}
              </td>
              <td className="px-4 py-2">
                <span className={`rounded px-2 py-0.5 text-xs ${
                  s.status === "new" ? "bg-blue-900 text-blue-200"
                  : s.status === "approved" ? "bg-green-900 text-green-200"
                  : s.status === "rejected" ? "bg-red-900 text-red-200"
                  : "bg-gray-800 text-gray-300"
                }`}>
                  {s.status}
                </span>
              </td>
              <td className="px-4 py-2 text-gray-500 text-xs whitespace-nowrap">
                {new Date(s.detected_at).toLocaleString()}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {rows.length === 0 && <p className="px-4 py-6 text-center text-gray-500">No signals found.</p>}
      <Pagination page={page} totalPages={totalPages} totalItems={data.length} perPage={PER_PAGE} onChange={setPage} />
    </>
  );
}

function OrdersTab({ data }: { data: any[] }) {
  const [page, setPage] = useState(1);
  const rows = data.slice((page - 1) * PER_PAGE, page * PER_PAGE);
  const totalPages = Math.max(1, Math.ceil(data.length / PER_PAGE));

  return (
    <>
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-gray-800 text-left text-gray-400 text-xs uppercase">
            <th className="px-4 py-2">Platform</th>
            <th className="px-4 py-2">Market</th>
            <th className="px-4 py-2">Side</th>
            <th className="px-4 py-2 text-right">Qty</th>
            <th className="px-4 py-2 text-right">Price</th>
            <th className="px-4 py-2">Status</th>
            <th className="px-4 py-2">Created</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((o: any) => (
            <tr key={o.id} className="border-b border-gray-800 hover:bg-gray-900">
              <td className="px-4 py-2 capitalize text-gray-400 text-xs">{o.platform}</td>
              <td className="px-4 py-2 max-w-[200px]">
                <span className="truncate block text-gray-200 text-xs font-mono" title={o.market_id}>
                  {o.market_id?.slice(0, 18) + "…" || "—"}
                </span>
              </td>
              <td className="px-4 py-2">
                <span className={`rounded px-1.5 py-0.5 text-xs font-medium ${
                  o.side === "buy" ? "bg-green-900/60 text-green-300" : "bg-red-900/60 text-red-300"
                }`}>
                  {o.side?.toUpperCase()}
                </span>
              </td>
              <td className="px-4 py-2 text-right text-gray-300 text-xs">
                {o.requested_contracts != null ? Number(o.requested_contracts).toFixed(2) : "—"}
              </td>
              <td className="px-4 py-2 text-right text-gray-300 text-xs">
                {o.price_limit != null ? `${(Number(o.price_limit) * 100).toFixed(0)}¢` : "—"}
              </td>
              <td className="px-4 py-2">
                <span className={`rounded px-2 py-0.5 text-xs ${
                  o.status === "filled" ? "bg-green-900 text-green-200"
                  : o.status === "pending" ? "bg-yellow-900 text-yellow-200"
                  : o.status === "cancelled" ? "bg-red-900 text-red-200"
                  : "bg-gray-800 text-gray-300"
                }`}>
                  {o.status}
                </span>
              </td>
              <td className="px-4 py-2 text-gray-500 text-xs whitespace-nowrap">
                {o.submitted_at ? new Date(o.submitted_at).toLocaleString() : "—"}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {rows.length === 0 && <p className="px-4 py-6 text-center text-gray-500">No orders found.</p>}
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
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-gray-800 text-left text-gray-400 text-xs uppercase">
            <th className="px-4 py-2">Market</th>
            <th className="px-4 py-2">Platform</th>
            <th className="px-4 py-2">Outcome</th>
            <th className="px-4 py-2 text-right">Contracts</th>
            <th className="px-4 py-2 text-right">Avg Price</th>
            <th className="px-4 py-2 text-right">Exposure</th>
            <th className="px-4 py-2 text-right">Realized P&L</th>
            <th className="px-4 py-2">Updated</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((p: any) => (
            <tr key={p.id} className="border-b border-gray-800 hover:bg-gray-900">
              <td className="px-4 py-2 max-w-[180px]">
                <span className="truncate block text-gray-200 text-xs" title={p.market_title || p.market_id}>
                  {p.market_title || p.market_id?.slice(0, 10) + "…" || "—"}
                </span>
              </td>
              <td className="px-4 py-2">
                <span className={`text-xs px-1.5 py-0.5 rounded font-medium ${
                  p.platform === "kalshi" ? "bg-blue-900/60 text-blue-300" : "bg-purple-900/60 text-purple-300"
                }`}>
                  {p.platform}
                </span>
              </td>
              <td className="px-4 py-2 text-gray-300 capitalize text-xs">{p.outcome}</td>
              <td className="px-4 py-2 text-right text-gray-300 text-xs">{Number(p.contracts).toFixed(2)}</td>
              <td className="px-4 py-2 text-right text-gray-300 text-xs">
                {(Number(p.avg_price) * 100).toFixed(0)}¢
              </td>
              <td className="px-4 py-2 text-right text-gray-300 text-xs">{fmt(Number(p.exposure))}</td>
              <td className={`px-4 py-2 text-right text-xs font-medium ${
                Number(p.realized_pnl) >= 0 ? "text-green-400" : "text-red-400"
              }`}>
                {fmt(Number(p.realized_pnl))}
              </td>
              <td className="px-4 py-2 text-gray-500 text-xs whitespace-nowrap">
                {new Date(p.updated_at).toLocaleString()}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {rows.length === 0 && <p className="px-4 py-6 text-center text-gray-500">No positions found.</p>}
      <Pagination page={page} totalPages={totalPages} totalItems={data.length} perPage={PER_PAGE} onChange={setPage} />
    </>
  );
}

// ── Page ──────────────────────────────────────────────────────────────────────
const TABS: { id: Tab; label: string }[] = [
  { id: "orders", label: "Orders" },
  { id: "positions", label: "Positions" },
];

export function TradeHistoryPage() {
  const [activeTab, setActiveTab] = useState<Tab>("orders");
  const { data: orders } = useOrders();
  const { data: positions } = usePositions();

  const allOrders = useMemo(
    () => [...(orders ?? [])].sort((a, b) => new Date(b.submitted_at ?? 0).getTime() - new Date(a.submitted_at ?? 0).getTime()),
    [orders]
  );
  const allPositions = useMemo(
    () => [...(positions ?? [])].sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()),
    [positions]
  );

  const counts: Record<Tab, number> = {
    orders: allOrders.length,
    positions: allPositions.length,
  };

  return (
    <div className="space-y-4">
      <h2 className="text-xl font-semibold">Trade History</h2>

      <ChartsPanel positions={allPositions} orders={allOrders} />

      <div className="rounded-lg border border-gray-800 bg-gray-950 overflow-x-auto">
        <div className="flex border-b border-gray-800">
          {TABS.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`px-5 py-3 text-sm font-medium transition-colors flex items-center gap-2 ${
                activeTab === tab.id
                  ? "border-b-2 border-blue-500 text-white"
                  : "text-gray-400 hover:text-gray-200"
              }`}
            >
              {tab.label}
              {counts[tab.id] > 0 && (
                <span className="rounded-full bg-gray-800 px-1.5 py-0.5 text-xs text-gray-400">
                  {counts[tab.id]}
                </span>
              )}
            </button>
          ))}
        </div>

        {activeTab === "orders" && <OrdersTab data={allOrders} />}
        {activeTab === "positions" && <PositionsTab data={allPositions} />}
      </div>
    </div>
  );
}
