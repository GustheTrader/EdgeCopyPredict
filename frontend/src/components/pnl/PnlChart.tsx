import { useEffect, useRef } from "react";
import { createChart, IChartApi, ISeriesApi, LineData } from "lightweight-charts";

interface PnlChartProps {
  data: Array<{ time: string; value: number }>;
  height?: number;
}

export function PnlChart({ data, height = 300 }: PnlChartProps) {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const seriesRef = useRef<ISeriesApi<"Line"> | null>(null);

  useEffect(() => {
    if (!chartContainerRef.current) return;

    const chart = createChart(chartContainerRef.current, {
      height,
      layout: {
        background: { color: "#0f1117" },
        textColor: "#d1d5db",
      },
      grid: {
        vertLines: { color: "#1f2937" },
        horzLines: { color: "#1f2937" },
      },
      rightPriceScale: {
        borderColor: "#374151",
      },
      timeScale: {
        borderColor: "#374151",
      },
    });
    chartRef.current = chart;

    const lineSeries = chart.addLineSeries({
      color: "#22c55e",
      lineWidth: 2,
      title: "Total P&L",
    });
    seriesRef.current = lineSeries;

    if (data.length > 0) {
      const chartData: LineData[] = data.map((d) => ({
        time: d.time as any,
        value: d.value,
      }));
      lineSeries.setData(chartData);
      chart.timeScale().fitContent();
    }

    const handleResize = () => {
      if (chartContainerRef.current) {
        chart.applyOptions({ width: chartContainerRef.current.clientWidth });
      }
    };
    window.addEventListener("resize", handleResize);

    return () => {
      window.removeEventListener("resize", handleResize);
      chart.remove();
    };
  }, [data, height]);

  return <div ref={chartContainerRef} className="w-full" />;
}
