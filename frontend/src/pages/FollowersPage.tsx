import { useState, useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import {
  useFollowers,
  useUpdateFollowerStatus,
  useUpdateFollowerWalletAddress,
  useUpdateFollowerPlatform,
  useUpdateFollowerRiskConfig,
  useCreateFollower,
  useDeleteFollower,
  usePnlSummary,
} from "@/hooks/useApi";
import { RiskConfigEditor } from "@/components/followers/RiskConfigEditor";

function medal(rank: number) {
  if (rank === 0) return "🥇";
  if (rank === 1) return "🥈";
  if (rank === 2) return "🥉";
  return `#${rank + 1}`;
}

function fmt(n: number) {
  const abs = Math.abs(n);
  const s = abs >= 1000 ? `$${(abs / 1000).toFixed(1)}k` : `$${abs.toFixed(2)}`;
  return n < 0 ? `-${s}` : `+${s}`;
}

export function FollowersPage() {
  const qc = useQueryClient();
  useEffect(() => {
    qc.invalidateQueries({ queryKey: ["followers"] });
    qc.invalidateQueries({ queryKey: ["pnl"] });
  }, []);

  const { data: followers } = useFollowers();
  const { data: pnl } = usePnlSummary();
  const updateStatus = useUpdateFollowerStatus();
  const updateWallet = useUpdateFollowerWalletAddress();
  const updatePlatform = useUpdateFollowerPlatform();
  const createFollower = useCreateFollower();
  const updateRiskConfig = useUpdateFollowerRiskConfig();
  const deleteFollower = useDeleteFollower();

  const [editing, setEditing] = useState<string | null>(null);
  const [editingWallet, setEditingWallet] = useState<string | null>(null);
  const [walletInput, setWalletInput] = useState("");
  const [newName, setNewName] = useState("");
  const [newWallet, setNewWallet] = useState("");
  const [newPlatform, setNewPlatform] = useState("polymarket");
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);

  const followerMap = new Map((followers ?? []).map((f: any) => [f.id, f]));

  const top5 = [...(pnl ?? [])]
    .sort((a, b) => b.total_pnl - a.total_pnl)
    .slice(0, 5)
    .map((p) => ({
      ...p,
      name: p.follower_account_id
        ? (followerMap.get(p.follower_account_id) as any)?.display_name ?? "Unknown"
        : "Paper",
    }));

  const maxAbsPnl = top5.reduce((acc, p) => Math.max(acc, Math.abs(p.total_pnl)), 0.01);

  return (
    <div className="space-y-6">
      <h2 className="text-xl font-semibold">Followers</h2>

      {/* Top 5 stats */}
      {top5.length > 0 && (
        <div className="rounded-lg border border-gray-800 bg-gray-950 p-4">
          <h3 className="mb-3 text-sm font-semibold text-gray-300">Top 5 Followers by PnL</h3>
          <div className="space-y-2">
            {top5.map((p, i) => {
              const pct = Math.round((Math.abs(p.total_pnl) / maxAbsPnl) * 100);
              const positive = p.total_pnl >= 0;
              return (
                <div key={p.follower_account_id ?? "paper"} className="flex items-center gap-3">
                  <span className="w-8 text-center text-sm">{medal(i)}</span>
                  <span className="w-32 truncate text-sm font-medium text-gray-200">{p.name}</span>
                  <div className="flex-1 h-2 rounded-full bg-gray-800">
                    <div
                      className={`h-2 rounded-full ${positive ? "bg-green-500" : "bg-red-500"}`}
                      style={{ width: `${pct}%` }}
                    />
                  </div>
                  <span className={`w-20 text-right text-xs font-mono font-semibold ${positive ? "text-green-400" : "text-red-400"}`}>
                    {fmt(p.total_pnl)}
                  </span>
                  <span className="w-12 text-right text-xs text-gray-500">
                    {(p.win_rate * 100).toFixed(0)}% wr
                  </span>
                  <span className="w-16 text-right text-xs text-gray-500">
                    {p.trade_count} trades
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Add form */}
      <div className="rounded-lg border border-gray-800 bg-gray-950 p-4">
        <h3 className="mb-2 text-sm font-medium text-gray-300">Add Follower</h3>
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
          <select
            value={newPlatform}
            onChange={(e) => setNewPlatform(e.target.value)}
            className="rounded bg-gray-900 px-3 py-1 text-sm text-white border border-gray-800"
          >
            <option value="polymarket">Polymarket</option>
            <option value="kalshi">Kalshi</option>
          </select>
          <button
            onClick={() => {
              if (!newName.trim()) return;
              createFollower.mutate({ display_name: newName, wallet_address: newWallet || undefined, platform: newPlatform });
              setNewName("");
              setNewWallet("");
              setNewPlatform("polymarket");
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
              <th className="px-4 py-2">Currency</th>
              <th className="px-4 py-2">Platform</th>
              <th className="px-4 py-2">Wallet</th>
              <th className="px-4 py-2">PnL</th>
              <th className="px-4 py-2">Actions</th>
            </tr>
          </thead>
          <tbody>
            {(followers || []).map((f: any) => {
              const fpnl = (pnl ?? []).find((p) => p.follower_account_id === f.id);
              return (
                <tr key={f.id} className="border-b border-gray-800 hover:bg-gray-900">
                  <td className="px-4 py-2 font-medium">{f.display_name}</td>
                  <td className="px-4 py-2">
                    <span className={`rounded px-2 py-0.5 text-xs ${
                      f.status === "active" ? "bg-green-900 text-green-200" : "bg-gray-800 text-gray-300"
                    }`}>
                      {f.status}
                    </span>
                  </td>
                  <td className="px-4 py-2">{f.base_currency}</td>
                  <td className="px-4 py-2">
                    <select
                      value={f.platform}
                      onChange={(e) => updatePlatform.mutate({ id: f.id, platform: e.target.value })}
                      className="rounded bg-gray-900 px-2 py-0.5 text-xs text-white border border-gray-800"
                    >
                      <option value="polymarket">Polymarket</option>
                      <option value="kalshi">Kalshi</option>
                    </select>
                  </td>
                  <td className="px-4 py-2">
                    {editingWallet === f.id ? (
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
                            updateWallet.mutate({ id: f.id, wallet_address: walletInput || undefined });
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
                        onClick={() => { setEditingWallet(f.id); setWalletInput(f.wallet_address || ""); }}
                      >
                        {f.wallet_address
                          ? `${f.wallet_address.slice(0, 8)}…${f.wallet_address.slice(-4)}`
                          : "—"}
                      </span>
                    )}
                  </td>
                  <td className="px-4 py-2">
                    {fpnl ? (
                      <span className={`text-xs font-mono font-semibold ${fpnl.total_pnl >= 0 ? "text-green-400" : "text-red-400"}`}>
                        {fmt(fpnl.total_pnl)}
                      </span>
                    ) : (
                      <span className="text-xs text-gray-600">—</span>
                    )}
                  </td>
                  <td className="px-4 py-2">
                    <div className="flex gap-1">
                      <button
                        onClick={() => updateStatus.mutate({ id: f.id, status: f.status === "active" ? "paused" : "active" })}
                        className="rounded bg-gray-700 px-2 py-1 text-xs text-white hover:bg-gray-600"
                      >
                        {f.status === "active" ? "Pause" : "Activate"}
                      </button>
                      <button
                        onClick={() => setEditing(editing === f.id ? null : f.id)}
                        className="rounded bg-blue-700 px-2 py-1 text-xs text-white hover:bg-blue-600"
                      >
                        Risk
                      </button>
                      {confirmDelete === f.id ? (
                        <>
                          <button
                            onClick={() => { deleteFollower.mutate(f.id); setConfirmDelete(null); }}
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
                          onClick={() => setConfirmDelete(f.id)}
                          className="rounded bg-red-900 px-2 py-1 text-xs text-red-300 hover:bg-red-800"
                        >
                          Delete
                        </button>
                      )}
                    </div>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {editing && followers?.find((f: any) => f.id === editing) && (
        <div className="mt-4">
          <RiskConfigEditor
            follower={followers.find((f: any) => f.id === editing)}
            onSave={(cfg) => { updateRiskConfig.mutate({ id: editing, config: cfg }); setEditing(null); }}
            onCancel={() => setEditing(null)}
          />
        </div>
      )}
    </div>
  );
}
