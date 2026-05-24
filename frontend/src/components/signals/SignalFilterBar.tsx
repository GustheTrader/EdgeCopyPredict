import { useState } from "react";
import { clsx } from "clsx";

interface SignalFilters {
  platform?: string;
  master_id?: string;
  status?: string;
  search?: string;
}

interface SignalFilterBarProps {
  filters: SignalFilters;
  onChange: (filters: SignalFilters) => void;
  masters: Array<{ id: string; display_name: string }>;
}

export function SignalFilterBar({ filters, onChange, masters }: SignalFilterBarProps) {
  const [local, setLocal] = useState(filters);

  const apply = () => onChange(local);
  const clear = () => {
    const empty: SignalFilters = {};
    setLocal(empty);
    onChange(empty);
  };

  return (
    <div className="flex flex-wrap items-center gap-2 rounded-lg border border-gray-700 bg-gray-950 p-3">
      <select
        className="rounded border border-gray-700 bg-gray-900 px-2 py-1 text-sm text-gray-300"
        value={local.platform || ""}
        onChange={(e) => setLocal({ ...local, platform: e.target.value || undefined })}
      >
        <option value="">All Platforms</option>
        <option value="polymarket">Polymarket</option>
        <option value="kalshi">Kalshi</option>
      </select>

      <select
        className="rounded border border-gray-700 bg-gray-900 px-2 py-1 text-sm text-gray-300"
        value={local.master_id || ""}
        onChange={(e) => setLocal({ ...local, master_id: e.target.value || undefined })}
      >
        <option value="">All Masters</option>
        {masters.map((m) => (
          <option key={m.id} value={m.id}>
            {m.display_name}
          </option>
        ))}
      </select>

      <select
        className="rounded border border-gray-700 bg-gray-900 px-2 py-1 text-sm text-gray-300"
        value={local.status || ""}
        onChange={(e) => setLocal({ ...local, status: e.target.value || undefined })}
      >
        <option value="">All Statuses</option>
        <option value="new">New</option>
        <option value="approved">Approved</option>
        <option value="rejected">Rejected</option>
        <option value="filtered">Filtered</option>
      </select>

      <input
        type="text"
        placeholder="Search markets..."
        className="rounded border border-gray-700 bg-gray-900 px-2 py-1 text-sm text-gray-300"
        value={local.search || ""}
        onChange={(e) => setLocal({ ...local, search: e.target.value || undefined })}
      />

      <button
        onClick={apply}
        className={clsx(
          "rounded px-3 py-1 text-sm font-medium",
          "bg-blue-600 text-white hover:bg-blue-500"
        )}
      >
        Apply
      </button>
      <button
        onClick={clear}
        className="rounded px-3 py-1 text-sm font-medium text-gray-400 hover:text-gray-200"
      >
        Clear
      </button>
    </div>
  );
}
