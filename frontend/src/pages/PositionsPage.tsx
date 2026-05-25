import { useState } from "react";
import { usePositions } from "@/hooks/useApi";
import { Pagination } from "@/components/ui/Pagination";

const PER_PAGE = 20;

function fmt(n: number, prefix = "$") {
  return `${prefix}${n.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
}

export function PositionsPage() {
  const { data: positions } = usePositions();
  const [page, setPage] = useState(1);

  const all = positions ?? [];
  const totalPages = Math.max(1, Math.ceil(all.length / PER_PAGE));
  const rows = all.slice((page - 1) * PER_PAGE, page * PER_PAGE);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold">Positions</h2>
        <span className="text-xs text-gray-500">{all.length} total</span>
      </div>
      <div className="rounded-lg border border-gray-800 bg-gray-950 overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-gray-800 text-left text-gray-400 text-xs uppercase">
              <th className="px-4 py-2">Market</th>
              <th className="px-4 py-2">Platform</th>
              <th className="px-4 py-2">Outcome</th>
              <th className="px-4 py-2 text-right">Contracts</th>
              <th className="px-4 py-2 text-right">Avg Price</th>
              <th className="px-4 py-2 text-right">Exposure</th>
              <th className="px-4 py-2 text-right">Unrealized P&L</th>
              <th className="px-4 py-2 text-right">Realized P&L</th>
              <th className="px-4 py-2">Updated</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((p: any) => (
              <tr key={p.id} className="border-b border-gray-800 hover:bg-gray-900">
                <td className="px-4 py-2 max-w-[200px]">
                  <span
                    className="truncate block text-gray-200 text-xs"
                    title={p.market_title || p.market_id}
                  >
                    {p.market_title || p.market_id?.slice(0, 10) + "…" || "—"}
                  </span>
                </td>
                <td className="px-4 py-2">
                  <span className={`text-xs px-1.5 py-0.5 rounded font-medium ${
                    p.platform === "kalshi"
                      ? "bg-blue-900/60 text-blue-300"
                      : "bg-purple-900/60 text-purple-300"
                  }`}>
                    {p.platform}
                  </span>
                </td>
                <td className="px-4 py-2 text-gray-300 capitalize text-xs">{p.outcome}</td>
                <td className="px-4 py-2 text-right text-gray-300 text-xs">
                  {Number(p.contracts).toFixed(2)}
                </td>
                <td className="px-4 py-2 text-right text-gray-300 text-xs">
                  {(Number(p.avg_price) * 100).toFixed(0)}¢
                </td>
                <td className="px-4 py-2 text-right text-gray-300 text-xs">
                  {fmt(Number(p.exposure))}
                </td>
                <td className={`px-4 py-2 text-right text-xs font-medium ${
                  Number(p.unrealized_pnl) >= 0 ? "text-green-400" : "text-red-400"
                }`}>
                  {fmt(Number(p.unrealized_pnl))}
                </td>
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
        {rows.length === 0 && (
          <p className="px-4 py-6 text-center text-gray-500">No positions found.</p>
        )}
        <Pagination
          page={page}
          totalPages={totalPages}
          totalItems={all.length}
          perPage={PER_PAGE}
          onChange={setPage}
        />
      </div>
    </div>
  );
}
