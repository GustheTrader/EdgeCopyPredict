import { useSystemStatus, useToggleSystem } from "@/hooks/useApi";

export function GlobalKillSwitch() {
  const { data: status } = useSystemStatus();
  const toggle = useToggleSystem();
  const liveTrading = status?.live_trading_enabled ?? false;

  return (
    <div className="flex items-center gap-2">
      <span className={`rounded px-2 py-0.5 text-xs font-medium ${
        liveTrading
          ? "bg-green-900 text-green-200"
          : "bg-yellow-900 text-yellow-200"
      }`}>
        {liveTrading ? "LIVE" : "DRY RUN"}
      </span>
      <button
        onClick={() => toggle.mutate()}
        disabled={toggle.isPending}
        className={`rounded px-3 py-1 text-xs font-medium text-white transition-colors ${
          toggle.isError
            ? "bg-red-800 hover:bg-red-700"
            : liveTrading
            ? "bg-red-700 hover:bg-red-600"
            : "bg-blue-700 hover:bg-blue-600"
        } disabled:opacity-50 disabled:cursor-not-allowed`}
        title={toggle.isError ? String(toggle.error) : undefined}
      >
        {toggle.isPending
          ? "Updating..."
          : toggle.isError
          ? "Failed — retry?"
          : liveTrading
          ? "Stop Copy"
          : "Resume Copy"}
      </button>
    </div>
  );
}
