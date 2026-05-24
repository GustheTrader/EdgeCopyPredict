import { useState, useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useAlerts, useAckAlert } from "@/hooks/useApi";

export function AlertsPage() {
  const qc = useQueryClient();
  useEffect(() => { qc.invalidateQueries({ queryKey: ["alerts"] }); }, []);

  const { data: alerts } = useAlerts();
  const ack = useAckAlert();
  const [expanded, setExpanded] = useState<Set<string>>(new Set());

  function toggle(id: string) {
    setExpanded((prev) => {
      const next = new Set(prev);
      next.has(id) ? next.delete(id) : next.add(id);
      return next;
    });
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold">Alerts</h2>
        {alerts && (
          <span className="text-xs text-gray-500">
            {alerts.filter((a: any) => !a.acked).length} open · {alerts.length} total
          </span>
        )}
      </div>

      <div className="rounded-lg border border-gray-800 bg-gray-950">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-gray-800 text-left text-gray-400">
              <th className="px-4 py-2">Severity</th>
              <th className="px-4 py-2">Type</th>
              <th className="px-4 py-2">Message</th>
              <th className="px-4 py-2">Status</th>
              <th className="px-4 py-2">Created</th>
              <th className="px-4 py-2">Actions</th>
            </tr>
          </thead>
          <tbody>
            {(alerts || []).map((a: any) => {
              const isExpanded = expanded.has(a.id);
              const isLong = a.message?.length > 80;
              return (
                <>
                  <tr key={a.id} className="border-b border-gray-800 hover:bg-gray-900">
                    <td className="px-4 py-2">
                      <span className={`rounded px-2 py-0.5 text-xs ${
                        a.severity === "critical" ? "bg-red-900 text-red-200" :
                        a.severity === "warning"  ? "bg-yellow-900 text-yellow-200" :
                        "bg-blue-900 text-blue-200"
                      }`}>
                        {a.severity}
                      </span>
                    </td>
                    <td className="px-4 py-2 text-gray-300">{a.category}</td>
                    <td className="px-4 py-2 max-w-xs">
                      <div className="flex items-start gap-2">
                        <span className={isExpanded ? "whitespace-pre-wrap break-words" : "truncate max-w-[260px] inline-block"}>
                          {a.message}
                        </span>
                        {isLong && (
                          <button
                            onClick={() => toggle(a.id)}
                            className="shrink-0 text-xs text-blue-400 hover:text-blue-300 underline"
                          >
                            {isExpanded ? "collapse" : "expand"}
                          </button>
                        )}
                      </div>
                    </td>
                    <td className="px-4 py-2">
                      <span className={`rounded px-2 py-0.5 text-xs ${
                        a.acked ? "bg-gray-800 text-gray-500" : "bg-yellow-900/60 text-yellow-300"
                      }`}>
                        {a.acked ? "acked" : "open"}
                      </span>
                    </td>
                    <td className="px-4 py-2 text-gray-400 whitespace-nowrap">
                      {new Date(a.created_at).toLocaleString()}
                    </td>
                    <td className="px-4 py-2">
                      {!a.acked && (
                        <button
                          onClick={() => ack.mutate(a.id)}
                          disabled={ack.isPending}
                          className="rounded bg-gray-700 px-2 py-1 text-xs text-white hover:bg-gray-600 disabled:opacity-50"
                        >
                          Ack
                        </button>
                      )}
                    </td>
                  </tr>
                  {isExpanded && (
                    <tr key={`${a.id}-expanded`} className="border-b border-gray-800 bg-gray-900/50">
                      <td colSpan={6} className="px-4 py-3">
                        <div className="rounded border border-gray-700 bg-gray-900 p-3">
                          <p className="text-xs font-semibold text-gray-400 mb-1 uppercase tracking-wide">Full Message</p>
                          <pre className="text-sm text-gray-200 whitespace-pre-wrap break-words font-mono">{a.message}</pre>
                        </div>
                      </td>
                    </tr>
                  )}
                </>
              );
            })}
          </tbody>
        </table>
        {(!alerts || alerts.length === 0) && (
          <p className="px-4 py-6 text-center text-gray-500">No alerts found.</p>
        )}
      </div>
    </div>
  );
}
