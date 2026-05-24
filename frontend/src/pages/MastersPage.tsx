import { useState, useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import {
  useMasters,
  useUpdateMasterStatus,
  useUpdateMasterWalletAddress,
  useCreateMaster,
  useDeleteMaster,
} from "@/hooks/useApi";

function medal(rank: number) {
  if (rank === 0) return "🥇";
  if (rank === 1) return "🥈";
  if (rank === 2) return "🥉";
  return `#${rank + 1}`;
}

export function MastersPage() {
  const qc = useQueryClient();
  useEffect(() => { qc.invalidateQueries({ queryKey: ["masters"] }); }, []);

  const { data: masters, isLoading } = useMasters();
  const updateStatus = useUpdateMasterStatus();
  const updateWallet = useUpdateMasterWalletAddress();
  const createMaster = useCreateMaster();
  const deleteMaster = useDeleteMaster();

  const [editingWallet, setEditingWallet] = useState<string | null>(null);
  const [walletInput, setWalletInput] = useState("");
  const [newName, setNewName] = useState("");
  const [newWallet, setNewWallet] = useState("");
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);

  const sorted = [...(masters ?? [])].sort((a: any, b: any) => {
    if (a.status === "active" && b.status !== "active") return -1;
    if (b.status === "active" && a.status !== "active") return 1;
    return (b.master_weight ?? 0) - (a.master_weight ?? 0);
  });
  const top5 = sorted.slice(0, 5);

  return (
    <div className="space-y-6">
      <h2 className="text-xl font-semibold">Masters</h2>

      {/* Top 5 stats */}
      {top5.length > 0 && (
        <div className="rounded-lg border border-gray-800 bg-gray-950 p-4">
          <h3 className="mb-3 text-sm font-semibold text-gray-300">Top Masters by Weight</h3>
          <div className="space-y-2">
            {top5.map((m: any, i: number) => {
              const weight = m.master_weight ?? 0;
              const maxWeight = (masters ?? []).reduce((acc: number, x: any) => Math.max(acc, x.master_weight ?? 0), 1);
              const pct = maxWeight > 0 ? Math.round((weight / maxWeight) * 100) : 0;
              return (
                <div key={m.id} className="flex items-center gap-3">
                  <span className="w-8 text-center text-sm">{medal(i)}</span>
                  <span className="w-32 truncate text-sm font-medium text-gray-200">{m.display_name}</span>
                  <div className="flex-1 h-2 rounded-full bg-gray-800">
                    <div
                      className={`h-2 rounded-full ${m.status === "active" ? "bg-blue-500" : "bg-gray-600"}`}
                      style={{ width: `${pct}%` }}
                    />
                  </div>
                  <span className="w-12 text-right text-xs text-gray-400">{weight}w</span>
                  <span className={`rounded px-2 py-0.5 text-xs ${
                    m.status === "active" ? "bg-green-900 text-green-200" : "bg-gray-800 text-gray-400"
                  }`}>
                    {m.status}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Add form */}
      <div className="rounded-lg border border-gray-800 bg-gray-950 p-4">
        <h3 className="mb-2 text-sm font-medium text-gray-300">Add Master</h3>
        <div className="flex gap-2">
          <input
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder="Display name"
            className="rounded bg-gray-900 px-3 py-1 text-sm text-white placeholder-gray-600 border border-gray-800 focus:border-blue-600 outline-none"
          />
          <input
            value={newWallet}
            onChange={(e) => setNewWallet(e.target.value)}
            placeholder="Wallet address (optional)"
            className="w-56 rounded bg-gray-900 px-3 py-1 text-sm text-white placeholder-gray-600 border border-gray-800 focus:border-blue-600 outline-none"
          />
          <button
            onClick={() => {
              if (!newName.trim()) return;
              createMaster.mutate({ display_name: newName, wallet_address: newWallet || undefined });
              setNewName("");
              setNewWallet("");
            }}
            className="rounded bg-blue-700 px-3 py-1 text-sm text-white hover:bg-blue-600"
          >
            Add
          </button>
        </div>
      </div>

      {/* Table */}
      <div className="rounded-lg border border-gray-800 bg-gray-950">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-gray-800 text-left text-gray-400">
              <th className="px-4 py-2">Name</th>
              <th className="px-4 py-2">Status</th>
              <th className="px-4 py-2">Weight</th>
              <th className="px-4 py-2">Wallet</th>
              <th className="px-4 py-2">Actions</th>
            </tr>
          </thead>
          <tbody>
            {(masters || []).map((m: any) => (
              <tr key={m.id} className="border-b border-gray-800 hover:bg-gray-900">
                <td className="px-4 py-2 font-medium">{m.display_name}</td>
                <td className="px-4 py-2">
                  <span className={`rounded px-2 py-0.5 text-xs ${
                    m.status === "active" ? "bg-green-900 text-green-200" : "bg-gray-800 text-gray-300"
                  }`}>
                    {m.status}
                  </span>
                </td>
                <td className="px-4 py-2">{m.master_weight}</td>
                <td className="px-4 py-2">
                  {editingWallet === m.id ? (
                    <div className="flex gap-1">
                      <input
                        value={walletInput}
                        onChange={(e) => setWalletInput(e.target.value)}
                        placeholder="0x..."
                        className="w-32 rounded bg-gray-900 px-2 py-0.5 text-xs text-white border border-gray-800"
                        autoFocus
                      />
                      <button
                        onClick={() => {
                          updateWallet.mutate({ id: m.id, wallet_address: walletInput || undefined });
                          setEditingWallet(null);
                        }}
                        className="rounded bg-blue-700 px-2 py-0.5 text-xs text-white"
                      >
                        Save
                      </button>
                      <button
                        onClick={() => setEditingWallet(null)}
                        className="rounded bg-gray-700 px-2 py-0.5 text-xs text-white"
                      >
                        Cancel
                      </button>
                    </div>
                  ) : (
                    <span
                      className="cursor-pointer text-gray-400 hover:text-white"
                      onClick={() => { setEditingWallet(m.id); setWalletInput(m.wallet_address || ""); }}
                    >
                      {m.wallet_address
                        ? `${m.wallet_address.slice(0, 8)}…${m.wallet_address.slice(-4)}`
                        : "—"}
                    </span>
                  )}
                </td>
                <td className="px-4 py-2">
                  <div className="flex gap-1">
                    <button
                      onClick={() => updateStatus.mutate({ id: m.id, status: m.status === "active" ? "paused" : "active" })}
                      className="rounded bg-gray-700 px-2 py-1 text-xs text-white hover:bg-gray-600"
                    >
                      {m.status === "active" ? "Pause" : "Activate"}
                    </button>
                    {confirmDelete === m.id ? (
                      <>
                        <button
                          onClick={() => { deleteMaster.mutate(m.id); setConfirmDelete(null); }}
                          className="rounded bg-red-700 px-2 py-1 text-xs text-white hover:bg-red-600"
                        >
                          Confirm
                        </button>
                        <button
                          onClick={() => setConfirmDelete(null)}
                          className="rounded bg-gray-700 px-2 py-1 text-xs text-white hover:bg-gray-600"
                        >
                          Cancel
                        </button>
                      </>
                    ) : (
                      <button
                        onClick={() => setConfirmDelete(m.id)}
                        className="rounded bg-red-900 px-2 py-1 text-xs text-red-300 hover:bg-red-800"
                      >
                        Delete
                      </button>
                    )}
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        {isLoading && <p className="px-4 py-6 text-center text-gray-500">Loading...</p>}
        {(!masters || masters.length === 0) && !isLoading && (
          <p className="px-4 py-6 text-center text-gray-500">No masters found.</p>
        )}
      </div>
    </div>
  );
}
