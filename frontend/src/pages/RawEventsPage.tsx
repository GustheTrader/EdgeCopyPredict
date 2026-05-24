import { useState } from "react";
import { useRawEvents } from "@/hooks/useApi";
import { Pagination } from "@/components/ui/Pagination";

const PER_PAGE = 25;

export function RawEventsPage() {
  const { data: events, isLoading } = useRawEvents();
  const [page, setPage] = useState(1);

  const all = events ?? [];
  const totalPages = Math.max(1, Math.ceil(all.length / PER_PAGE));
  const rows = all.slice((page - 1) * PER_PAGE, page * PER_PAGE);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold">Raw Events</h2>
        <span className="text-xs text-gray-500">{all.length} total</span>
      </div>
      <div className="rounded-lg border border-gray-800 bg-gray-950 overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-gray-800 text-left text-gray-400 text-xs uppercase">
              <th className="px-4 py-2">Platform</th>
              <th className="px-4 py-2">Source</th>
              <th className="px-4 py-2">External ID</th>
              <th className="px-4 py-2">Status</th>
              <th className="px-4 py-2">Detected</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((e: any) => (
              <tr key={e.id} className="border-b border-gray-800 hover:bg-gray-900">
                <td className="px-4 py-2 capitalize text-gray-300 text-xs">{e.platform}</td>
                <td className="px-4 py-2 text-gray-400 text-xs">{e.source}</td>
                <td className="px-4 py-2 font-mono text-xs text-gray-500 max-w-[260px] truncate">
                  {e.external_event_id}
                </td>
                <td className="px-4 py-2">
                  <span className={`rounded px-2 py-0.5 text-xs ${
                    e.status === "new"
                      ? "bg-yellow-900 text-yellow-200"
                      : e.status === "processed"
                      ? "bg-green-900 text-green-200"
                      : "bg-gray-800 text-gray-300"
                  }`}>
                    {e.status}
                  </span>
                </td>
                <td className="px-4 py-2 text-gray-500 text-xs whitespace-nowrap">
                  {new Date(e.detected_at).toLocaleString()}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        {isLoading && <p className="px-4 py-6 text-center text-gray-500">Loading…</p>}
        {all.length === 0 && !isLoading && (
          <p className="px-4 py-6 text-center text-gray-500">No raw events found.</p>
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
