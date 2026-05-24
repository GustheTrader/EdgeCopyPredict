import { useState, useMemo } from "react";
import { useSignalsList, useMasters } from "@/hooks/useApi";
import { useSignals } from "@/hooks/useSignals";
import { SignalFilterBar } from "@/components/signals/SignalFilterBar";
import { exportSignalsToCsv } from "@/utils/csvExport";
import { Pagination } from "@/components/ui/Pagination";

const PER_PAGE = 20;

function statusClass(status: string) {
  if (status === "new") return "bg-blue-900 text-blue-200";
  if (status === "approved") return "bg-green-900 text-green-200";
  if (status === "rejected") return "bg-red-900 text-red-200";
  return "bg-gray-800 text-gray-300";
}

export function SignalsPage() {
  const { data: restSignals } = useSignalsList();
  const { data: masters } = useMasters();
  const liveSignals = useSignals();
  const [filters, setFilters] = useState<any>({});
  const [page, setPage] = useState(1);
  const [copied, setCopied] = useState<string | null>(null);

  // Merge live SSE signals on top of REST results, deduped by id, then apply filters
  const allSignals = useMemo(() => {
    const map = new Map<string, any>();
    for (const s of restSignals ?? []) map.set(s.id, s);
    for (const s of liveSignals) map.set(s.id, s);
    let arr = Array.from(map.values()).sort(
      (a, b) => new Date(b.detected_at).getTime() - new Date(a.detected_at).getTime()
    );
    if (filters.platform) arr = arr.filter((s) => s.platform === filters.platform);
    if (filters.master_id) arr = arr.filter((s) => s.master_id === filters.master_id);
    if (filters.status) arr = arr.filter((s) => s.status === filters.status);
    if (filters.search) {
      const q = filters.search.toLowerCase();
      arr = arr.filter((s) =>
        (s.market_title ?? "").toLowerCase().includes(q) ||
        (s.outcome ?? "").toLowerCase().includes(q) ||
        (s.signal_platform_market_id ?? "").toLowerCase().includes(q)
      );
    }
    return arr;
  }, [restSignals, liveSignals, filters]);

  const totalPages = Math.max(1, Math.ceil(allSignals.length / PER_PAGE));
  const signals = allSignals.slice((page - 1) * PER_PAGE, page * PER_PAGE);

  function formatWallet(wallet?: string): string {
    if (!wallet) return "—";
    const w = wallet.toLowerCase();
    if (w.length <= 12) return w;
    return `${w.slice(0, 6)}…${w.slice(-4)}`;
  }

  function copyWallet(wallet: string) {
    navigator.clipboard.writeText(wallet).then(() => {
      setCopied(wallet);
      setTimeout(() => setCopied(null), 1500);
    });
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <h2 className="text-xl font-semibold">Signals</h2>
          {liveSignals.length > 0 && (
            <span className="flex items-center gap-1.5 text-xs text-green-400">
              <span className="h-1.5 w-1.5 rounded-full bg-green-400 animate-pulse" />
              Live
            </span>
          )}
        </div>
        <button
          onClick={() => signals.length > 0 && exportSignalsToCsv(signals)}
          className="rounded bg-gray-700 px-3 py-1 text-sm text-white hover:bg-gray-600"
        >
          Export CSV
        </button>
      </div>
      <SignalFilterBar filters={filters} onChange={(f) => { setFilters(f); setPage(1); }} masters={masters ?? []} />
      <div className="rounded-lg border border-gray-800 bg-gray-950 overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-gray-800 text-left text-gray-400">
              <th className="px-4 py-2">Market / Pair</th>
              <th className="px-4 py-2">Outcome</th>
              <th className="px-4 py-2">Platform</th>
              <th className="px-4 py-2">Side / Effect</th>
              <th className="px-4 py-2">Contracts</th>
              <th className="px-4 py-2">Price</th>
              <th className="px-4 py-2">Notional</th>
              <th className="px-4 py-2">Wallet</th>
              <th className="px-4 py-2">Status</th>
              <th className="px-4 py-2">Detected</th>
            </tr>
          </thead>
          <tbody>
            {signals.map((s: any) => (
              <tr key={s.id} className="border-b border-gray-800 hover:bg-gray-900">
                <td className="px-4 py-2 max-w-[200px]">
                  {s.market_title ? (
                    <span className="truncate block text-gray-200" title={s.market_title}>
                      {s.market_title}
                    </span>
                  ) : (
                    <span className="text-gray-500 text-xs font-mono">
                      {s.market_id ? s.market_id.slice(0, 8) + "…" : "—"}
                    </span>
                  )}
                </td>
                <td className="px-4 py-2 text-gray-300">{s.outcome}</td>
                <td className="px-4 py-2 capitalize text-gray-400">{s.platform}</td>
                <td className="px-4 py-2">
                  <div className="flex flex-col gap-0.5">
                    <span className={`rounded px-1.5 py-0.5 text-xs font-medium w-fit ${
                      s.side === "buy" ? "bg-green-900/60 text-green-300" : "bg-red-900/60 text-red-300"
                    }`}>
                      {s.side?.toUpperCase()}
                    </span>
                    <span className={`rounded px-1.5 py-0.5 text-xs w-fit ${
                      s.position_effect === "open"
                        ? "bg-blue-900/50 text-blue-300"
                        : "bg-orange-900/50 text-orange-300"
                    }`}>
                      {s.position_effect?.toUpperCase() ?? "—"}
                    </span>
                  </div>
                </td>
                <td className="px-4 py-2 text-gray-300">{s.size_contracts?.toFixed(2)}</td>
                <td className="px-4 py-2 text-gray-300">{s.entry_price != null ? `${(s.entry_price * 100).toFixed(0)}¢` : "—"}</td>
                <td className="px-4 py-2 text-gray-300">${s.notional?.toFixed(2)}</td>
                <td className="px-4 py-2">
                  {s.master_wallet ? (
                    <button
                      title={`${s.master_wallet}\nClick to copy`}
                      onClick={() => copyWallet(s.master_wallet)}
                      className="group flex items-center gap-1.5 rounded bg-indigo-900/40 border border-indigo-700/50 px-2 py-0.5 font-mono text-xs text-indigo-300 hover:bg-indigo-800/60 hover:text-indigo-200 transition-colors"
                    >
                      <span className="h-1.5 w-1.5 rounded-full bg-indigo-400 shrink-0" />
                      {copied === s.master_wallet ? (
                        <span className="text-green-400">Copied!</span>
                      ) : (
                        formatWallet(s.master_wallet)
                      )}
                    </button>
                  ) : (
                    <span className="text-gray-600 text-xs">—</span>
                  )}
                </td>
                <td className="px-4 py-2">
                  <span className={`rounded px-2 py-0.5 text-xs ${statusClass(s.status)}`}>
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
        {signals.length === 0 && (
          <p className="px-4 py-6 text-center text-gray-500">No signals yet. Waiting for live data…</p>
        )}
        <Pagination
          page={page}
          totalPages={totalPages}
          totalItems={allSignals.length}
          perPage={PER_PAGE}
          onChange={(p) => setPage(p)}
        />
      </div>
    </div>
  );
}
