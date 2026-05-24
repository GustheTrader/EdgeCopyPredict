export function downloadCsv(filename: string, rows: string[][]) {
  const csvContent = rows
    .map((row) =>
      row
        .map((cell) => {
          const escaped = ("" + cell).replace(/"/g, '""');
          if (escaped.includes(",") || escaped.includes('"') || escaped.includes("\n")) {
            return '"' + escaped + '"';
          }
          return escaped;
        })
        .join(",")
    )
    .join("\n");

  const blob = new Blob([csvContent], { type: "text/csv;charset=utf-8;" });
  const link = document.createElement("a");
  link.href = URL.createObjectURL(blob);
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
}

export function exportSignalsToCsv(signals: any[]) {
  const headers = [
    "ID",
    "Platform",
    "Master ID",
    "Outcome",
    "Side",
    "Size",
    "Notional",
    "Price",
    "Status",
    "Detected At",
  ];
  const rows = signals.map((s) => [
    s.id,
    s.platform,
    s.master_id,
    s.outcome,
    s.side,
    s.size_contracts,
    s.notional,
    s.entry_price,
    s.status,
    s.detected_at,
  ]);
  downloadCsv(`signals_${new Date().toISOString().slice(0, 10)}.csv`, [
    headers,
    ...rows,
  ]);
}

export function exportFillsToCsv(fills: any[]) {
  const headers = [
    "ID",
    "Order ID",
    "Platform",
    "Market",
    "Outcome",
    "Side",
    "Price",
    "Contracts",
    "Notional",
    "Fee",
    "Filled At",
  ];
  const rows = fills.map((f) => [
    f.id,
    f.order_id,
    f.platform,
    f.market_title || "",
    f.outcome,
    f.side,
    f.price,
    f.contracts,
    f.notional,
    f.fee,
    f.filled_at,
  ]);
  downloadCsv(`fills_${new Date().toISOString().slice(0, 10)}.csv`, [
    headers,
    ...rows,
  ]);
}
