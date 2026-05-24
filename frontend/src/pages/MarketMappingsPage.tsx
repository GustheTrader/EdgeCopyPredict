import { useState } from "react";
import {
  useMarketMappings,
  useCreateMarketMapping,
  useUpdateMarketMapping,
  useUpdateMarketMappingEnabled,
  useDeleteMarketMapping,
} from "@/hooks/useApi";

export function MarketMappingsPage() {
  const { data: mappings, isLoading } = useMarketMappings();
  const create = useCreateMarketMapping();
  const update = useUpdateMarketMapping();
  const updateEnabled = useUpdateMarketMappingEnabled();
  const remove = useDeleteMarketMapping();

  const [editing, setEditing] = useState<string | null>(null);
  const [showAdd, setShowAdd] = useState(false);

  const [form, setForm] = useState({
    polymarket_condition_id: "",
    polymarket_slug: "",
    kalshi_ticker: "",
    kalshi_event_ticker: "",
    confidence: "manual",
    enabled: false,
  });

  const resetForm = () =>
    setForm({
      polymarket_condition_id: "",
      polymarket_slug: "",
      kalshi_ticker: "",
      kalshi_event_ticker: "",
      confidence: "manual",
      enabled: false,
    });

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold">Market Mappings</h2>
        <button
          onClick={() => { setShowAdd(!showAdd); resetForm(); }}
          className="rounded bg-blue-700 px-3 py-1 text-sm text-white hover:bg-blue-600"
        >
          {showAdd ? "Cancel" : "Add Mapping"}
        </button>
      </div>

      {showAdd && (
        <div className="rounded-lg border border-gray-800 bg-gray-950 p-4 space-y-3">
          <h3 className="text-sm font-medium text-gray-300">New Market Mapping</h3>
          <div className="grid grid-cols-2 gap-3">
            <input
              value={form.polymarket_condition_id}
              onChange={(e) => setForm({ ...form, polymarket_condition_id: e.target.value })}
              placeholder="Polymarket condition ID"
              className="rounded bg-gray-900 px-3 py-1 text-sm text-white placeholder-gray-600 border border-gray-800 focus:border-blue-600 outline-none"
            />
            <input
              value={form.polymarket_slug}
              onChange={(e) => setForm({ ...form, polymarket_slug: e.target.value })}
              placeholder="Polymarket slug (optional)"
              className="rounded bg-gray-900 px-3 py-1 text-sm text-white placeholder-gray-600 border border-gray-800 focus:border-blue-600 outline-none"
            />
            <input
              value={form.kalshi_ticker}
              onChange={(e) => setForm({ ...form, kalshi_ticker: e.target.value })}
              placeholder="Kalshi ticker"
              className="rounded bg-gray-900 px-3 py-1 text-sm text-white placeholder-gray-600 border border-gray-800 focus:border-blue-600 outline-none"
            />
            <input
              value={form.kalshi_event_ticker}
              onChange={(e) => setForm({ ...form, kalshi_event_ticker: e.target.value })}
              placeholder="Kalshi event ticker (optional)"
              className="rounded bg-gray-900 px-3 py-1 text-sm text-white placeholder-gray-600 border border-gray-800 focus:border-blue-600 outline-none"
            />
            <select
              value={form.confidence}
              onChange={(e) => setForm({ ...form, confidence: e.target.value })}
              className="rounded bg-gray-900 px-3 py-1 text-sm text-white border border-gray-800"
            >
              <option value="manual">Manual</option>
              <option value="high">High</option>
              <option value="medium">Medium</option>
              <option value="low">Low</option>
            </select>
            <label className="flex items-center gap-2 text-sm text-gray-300">
              <input
                type="checkbox"
                checked={form.enabled}
                onChange={(e) => setForm({ ...form, enabled: e.target.checked })}
                className="rounded bg-gray-900 border border-gray-800"
              />
              Enabled
            </label>
          </div>
          <button
            onClick={() => {
              if (!form.polymarket_condition_id || !form.kalshi_ticker) return;
              create.mutate({
                polymarket_condition_id: form.polymarket_condition_id,
                polymarket_slug: form.polymarket_slug || undefined,
                kalshi_ticker: form.kalshi_ticker,
                kalshi_event_ticker: form.kalshi_event_ticker || undefined,
                confidence: form.confidence,
                enabled: form.enabled,
              });
              setShowAdd(false);
              resetForm();
            }}
            className="rounded bg-blue-700 px-3 py-1 text-sm text-white hover:bg-blue-600"
          >
            Create
          </button>
        </div>
      )}

      <div className="rounded-lg border border-gray-800 bg-gray-950">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-gray-800 text-left text-gray-400">
              <th className="px-4 py-2">Polymarket</th>
              <th className="px-4 py-2">Kalshi</th>
              <th className="px-4 py-2">Confidence</th>
              <th className="px-4 py-2">Status</th>
              <th className="px-4 py-2">Actions</th>
            </tr>
          </thead>
          <tbody>
            {(mappings || []).map((m: any) => (
              <tr key={m.id} className="border-b border-gray-800 hover:bg-gray-900">
                <td className="px-4 py-2">
                  {editing === m.id ? (
                    <div className="space-y-1">
                      <input
                        value={form.polymarket_condition_id}
                        onChange={(e) => setForm({ ...form, polymarket_condition_id: e.target.value })}
                        className="w-full rounded bg-gray-900 px-2 py-0.5 text-xs text-white border border-gray-800"
                      />
                      <input
                        value={form.polymarket_slug}
                        onChange={(e) => setForm({ ...form, polymarket_slug: e.target.value })}
                        placeholder="slug"
                        className="w-full rounded bg-gray-900 px-2 py-0.5 text-xs text-white border border-gray-800"
                      />
                    </div>
                  ) : (
                    <div>
                      <div className="font-mono text-xs text-gray-400">{m.polymarket_condition_id}</div>
                      {m.polymarket_slug && <div className="text-xs text-gray-500">{m.polymarket_slug}</div>}
                    </div>
                  )}
                </td>
                <td className="px-4 py-2">
                  {editing === m.id ? (
                    <div className="space-y-1">
                      <input
                        value={form.kalshi_ticker}
                        onChange={(e) => setForm({ ...form, kalshi_ticker: e.target.value })}
                        className="w-full rounded bg-gray-900 px-2 py-0.5 text-xs text-white border border-gray-800"
                      />
                      <input
                        value={form.kalshi_event_ticker}
                        onChange={(e) => setForm({ ...form, kalshi_event_ticker: e.target.value })}
                        placeholder="event ticker"
                        className="w-full rounded bg-gray-900 px-2 py-0.5 text-xs text-white border border-gray-800"
                      />
                    </div>
                  ) : (
                    <div>
                      <div className="font-medium">{m.kalshi_ticker}</div>
                      {m.kalshi_event_ticker && <div className="text-xs text-gray-500">{m.kalshi_event_ticker}</div>}
                    </div>
                  )}
                </td>
                <td className="px-4 py-2">
                  {editing === m.id ? (
                    <select
                      value={form.confidence}
                      onChange={(e) => setForm({ ...form, confidence: e.target.value })}
                      className="rounded bg-gray-900 px-2 py-0.5 text-xs text-white border border-gray-800"
                    >
                      <option value="manual">Manual</option>
                      <option value="high">High</option>
                      <option value="medium">Medium</option>
                      <option value="low">Low</option>
                    </select>
                  ) : (
                    <span className="text-xs text-gray-400">{m.confidence}</span>
                  )}
                </td>
                <td className="px-4 py-2">
                  <button
                    onClick={() => updateEnabled.mutate({ id: m.id, enabled: !m.enabled })}
                    className={`rounded px-2 py-0.5 text-xs ${
                      m.enabled
                        ? "bg-green-900 text-green-200"
                        : "bg-gray-800 text-gray-300"
                    }`}
                  >
                    {m.enabled ? "Enabled" : "Disabled"}
                  </button>
                </td>
                <td className="px-4 py-2 space-x-2">
                  {editing === m.id ? (
                    <>
                      <button
                        onClick={() => {
                          update.mutate({
                            id: m.id,
                            polymarket_condition_id: form.polymarket_condition_id,
                            polymarket_slug: form.polymarket_slug || undefined,
                            kalshi_ticker: form.kalshi_ticker,
                            kalshi_event_ticker: form.kalshi_event_ticker || undefined,
                            confidence: form.confidence,
                            enabled: form.enabled,
                          });
                          setEditing(null);
                        }}
                        className="rounded bg-blue-700 px-2 py-0.5 text-xs text-white"
                      >
                        Save
                      </button>
                      <button
                        onClick={() => setEditing(null)}
                        className="rounded bg-gray-700 px-2 py-0.5 text-xs text-white"
                      >
                        Cancel
                      </button>
                    </>
                  ) : (
                    <>
                      <button
                        onClick={() => {
                          setEditing(m.id);
                          setForm({
                            polymarket_condition_id: m.polymarket_condition_id,
                            polymarket_slug: m.polymarket_slug || "",
                            kalshi_ticker: m.kalshi_ticker,
                            kalshi_event_ticker: m.kalshi_event_ticker || "",
                            confidence: m.confidence,
                            enabled: m.enabled,
                          });
                        }}
                        className="rounded bg-gray-700 px-2 py-0.5 text-xs text-white hover:bg-gray-600"
                      >
                        Edit
                      </button>
                      <button
                        onClick={() => {
                          if (confirm("Delete this mapping?")) remove.mutate(m.id);
                        }}
                        className="rounded bg-red-800 px-2 py-0.5 text-xs text-white hover:bg-red-700"
                      >
                        Delete
                      </button>
                    </>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        {isLoading && <p className="px-4 py-6 text-center text-gray-500">Loading...</p>}
        {(!mappings || mappings.length === 0) && !isLoading && (
          <p className="px-4 py-6 text-center text-gray-500">No market mappings found.</p>
        )}
      </div>
    </div>
  );
}
