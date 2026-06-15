import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";

export interface SignalDto {
  id: string;
  platform: string;
  master_id: string;
  master_wallet?: string;
  market_id?: string;
  market_title?: string;
  outcome: string;
  side: string;
  position_effect: string;
  size_contracts: number;
  notional: number;
  currency: string;
  entry_price: number;
  event_timestamp: string;
  detected_at: string;
  confidence_score?: number;
  status: string;
}

function signalTime(signal: SignalDto) {
  const ts = new Date(signal.detected_at).getTime();
  return Number.isFinite(ts) ? ts : 0;
}

function mergeSignal(signal: SignalDto, existing: SignalDto[] = []) {
  const map = new Map<string, SignalDto>();
  map.set(signal.id, signal);
  for (const item of existing) {
    if (!map.has(item.id)) map.set(item.id, item);
  }
  return Array.from(map.values())
    .sort((a, b) => signalTime(b) - signalTime(a))
    .slice(0, 200);
}

export function useSignals() {
  const [signals, setSignals] = useState<SignalDto[]>([]);
  const queryClient = useQueryClient();

  useEffect(() => {
    const sseBase = (import.meta.env.VITE_SSE_BASE_URL ?? "").replace(/\/$/, "");
    const source = new EventSource(`${sseBase}/sse/activity`);
    source.onmessage = (event) => {
      try {
        const signal: SignalDto = JSON.parse(event.data);
        setSignals((prev) => mergeSignal(signal, prev));
        queryClient.setQueriesData<SignalDto[]>(
          { queryKey: ["signals"] },
          (old) => mergeSignal(signal, old ?? []),
        );
        queryClient.invalidateQueries({ queryKey: ["system-status"] });
      } catch {
        /* ignore malformed */
      }
    };
    source.onerror = () => {
      queryClient.invalidateQueries({ queryKey: ["signals"] });
    };
    return () => source.close();
  }, [queryClient]);

  return signals;
}
