interface PaginationProps {
  page: number;
  totalPages: number;
  totalItems: number;
  perPage: number;
  onChange: (page: number) => void;
}

export function Pagination({ page, totalPages, totalItems, perPage, onChange }: PaginationProps) {
  if (totalPages <= 1) return null;

  const from = (page - 1) * perPage + 1;
  const to = Math.min(page * perPage, totalItems);

  const pages: (number | "…")[] = [];
  if (totalPages <= 7) {
    for (let i = 1; i <= totalPages; i++) pages.push(i);
  } else {
    pages.push(1);
    if (page > 3) pages.push("…");
    for (let i = Math.max(2, page - 1); i <= Math.min(totalPages - 1, page + 1); i++) pages.push(i);
    if (page < totalPages - 2) pages.push("…");
    pages.push(totalPages);
  }

  return (
    <div className="flex items-center justify-between px-4 py-3 border-t border-gray-800 text-xs text-gray-400">
      <span>{from}–{to} of {totalItems}</span>
      <div className="flex items-center gap-1">
        <button
          onClick={() => onChange(page - 1)}
          disabled={page === 1}
          className="px-2 py-1 rounded hover:bg-gray-800 disabled:opacity-30 disabled:cursor-not-allowed"
        >
          ‹
        </button>
        {pages.map((p, i) =>
          p === "…" ? (
            <span key={`ellipsis-${i}`} className="px-1">…</span>
          ) : (
            <button
              key={p}
              onClick={() => onChange(p as number)}
              className={`w-7 h-7 rounded text-xs font-medium ${
                p === page
                  ? "bg-blue-700 text-white"
                  : "hover:bg-gray-800 text-gray-400"
              }`}
            >
              {p}
            </button>
          )
        )}
        <button
          onClick={() => onChange(page + 1)}
          disabled={page === totalPages}
          className="px-2 py-1 rounded hover:bg-gray-800 disabled:opacity-30 disabled:cursor-not-allowed"
        >
          ›
        </button>
      </div>
    </div>
  );
}
