import { useEffect, useState } from "react";

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

export function useSignals() {
  const [signals, setSignals] = useState<SignalDto[]>([]);

  useEffect(() => {
    const sseBase = import.meta.env.VITE_SSE_BASE_URL ?? "";
    const source = new EventSource(`${sseBase}/sse/activity`);
    source.onmessage = (event) => {
      try {
        const signal: SignalDto = JSON.parse(event.data);
        setSignals((prev) => [signal, ...prev].slice(0, 200));
      } catch {
        /* ignore malformed */
      }
    };
    source.onerror = () => {
      /* auto-reconnects */
    };
    return () => source.close();
  }, []);

  return signals;
}
