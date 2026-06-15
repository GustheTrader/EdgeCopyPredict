import { useMemo, useState } from "react";
import { useOrders, usePaperWalletMovements, usePositions } from "@/hooks/useApi";

const HISTORY_LIMIT = 1000;
const DAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

type PlatformFilter = "all" | "polymarket" | "kalshi";

type DailyStat = {
  pnl: number;
  realized: number;
  unrealized: number;
  exposure: number;
  positions: number;
  orders: number;
  movements: number;
  cashFlow: number;
};

type CalendarCell = {
  iso: string;
  day: number;
  otherMonth: boolean;
};

const money = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
});

function fmtMoney(n: number) {
  return money.format(n);
}

function fmtCompact(n: number) {
  const abs = Math.abs(n);
  const sign = n < 0 ? "-" : "";
  if (abs >= 1000) return `${sign}$${(abs / 1000).toFixed(1)}k`;
  return fmtMoney(n);
}

function localIsoDate(d: Date) {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

function isoFor(y: number, monthIndex: number, day: number) {
  return `${y}-${String(monthIndex + 1).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
}

function labelDate(iso: string) {
  return new Date(`${iso}T12:00:00`).toLocaleDateString("en-US", {
    weekday: "long",
    month: "long",
    day: "numeric",
    year: "numeric",
  });
}

function moneyColor(n: number) {
  if (n > 0) return "text-emerald-300";
  if (n < 0) return "text-rose-300";
  return "text-gray-100";
}

function positionPnl(position: any) {
  return Number(position.realized_pnl ?? 0) + Number(position.unrealized_pnl ?? 0);
}

function emptyStat(): DailyStat {
  return {
    pnl: 0,
    realized: 0,
    unrealized: 0,
    exposure: 0,
    positions: 0,
    orders: 0,
    movements: 0,
    cashFlow: 0,
  };
}

function StatCard({
  label,
  value,
  sub,
  valueClass = "text-gray-100",
}: {
  label: string;
  value: string;
  sub?: string;
  valueClass?: string;
}) {
  return (
    <div className="rounded-lg border border-gray-800 bg-gray-950 px-4 py-3">
      <div className="text-[10px] font-semibold uppercase tracking-wide text-gray-500">{label}</div>
      <div className={`mt-1 text-xl font-semibold tabular-nums ${valueClass}`}>{value}</div>
      {sub && <div className="mt-1 text-xs text-gray-600">{sub}</div>}
    </div>
  );
}

function PlatformTabs({
  value,
  onChange,
}: {
  value: PlatformFilter;
  onChange: (value: PlatformFilter) => void;
}) {
  return (
    <div className="inline-flex rounded-lg border border-gray-800 bg-gray-950 p-1">
      {(["all", "polymarket", "kalshi"] as const).map((platform) => (
        <button
          key={platform}
          onClick={() => onChange(platform)}
          className={`rounded-md px-3 py-1.5 text-xs font-medium capitalize transition-colors ${
            value === platform
              ? "bg-sky-500 text-white"
              : "text-gray-500 hover:bg-gray-900 hover:text-gray-300"
          }`}
        >
          {platform}
        </button>
      ))}
    </div>
  );
}

function DayCell({
  cell,
  stat,
  isToday,
  selected,
  onClick,
}: {
  cell: CalendarCell;
  stat?: DailyStat;
  isToday: boolean;
  selected: boolean;
  onClick: () => void;
}) {
  const hasPositionPnl = !!stat && stat.positions > 0;
  const hasActivity = !!stat && (stat.positions > 0 || stat.orders > 0 || stat.movements > 0);
  const pnl = stat?.pnl ?? 0;
  const positive = pnl >= 0;

  let tone = "bg-gray-950 hover:bg-gray-900";
  if (cell.otherMonth) tone = "bg-gray-950/60";
  else if (hasPositionPnl && positive) tone = "bg-emerald-950/40 hover:bg-emerald-950/70";
  else if (hasPositionPnl) tone = "bg-rose-950/40 hover:bg-rose-950/70";
  else if (hasActivity) tone = "bg-amber-950/20 hover:bg-amber-950/40";

  let border = "border-gray-800";
  if (selected) border = "border-sky-400";
  else if (isToday) border = "border-sky-700";

  return (
    <button
      onClick={onClick}
      className={`min-h-[96px] rounded-lg border ${border} ${tone} p-2 text-left transition-colors`}
    >
      <div className="flex items-center justify-between gap-1">
        <span
          className={`text-xs font-semibold ${
            cell.otherMonth ? "text-gray-700" : isToday ? "text-sky-300" : "text-gray-400"
          }`}
        >
          {cell.day}
        </span>
        {isToday && !cell.otherMonth && (
          <span className="rounded bg-sky-500 px-1.5 py-0.5 text-[9px] font-bold uppercase text-white">
            Today
          </span>
        )}
      </div>

      {!cell.otherMonth && hasPositionPnl && (
        <div className="mt-3">
          <div className={`text-sm font-semibold leading-tight tabular-nums ${positive ? "text-emerald-300" : "text-rose-300"}`}>
            {fmtCompact(pnl)}
          </div>
          <div className="mt-1 text-[10px] text-gray-500">
            {stat.positions} position{stat.positions === 1 ? "" : "s"}
          </div>
        </div>
      )}

      {!cell.otherMonth && !hasPositionPnl && hasActivity && (
        <div className="mt-3 text-xs font-medium text-amber-300">Activity</div>
      )}

      {!cell.otherMonth && hasActivity && (
        <div className="mt-3 flex flex-wrap gap-1">
          {stat.orders > 0 && (
            <span className="rounded bg-gray-900 px-1.5 py-0.5 text-[9px] text-gray-400">
              {stat.orders} order{stat.orders === 1 ? "" : "s"}
            </span>
          )}
          {stat.movements > 0 && (
            <span className="rounded bg-gray-900 px-1.5 py-0.5 text-[9px] text-gray-400">
              {stat.movements} cash
            </span>
          )}
        </div>
      )}

      {!cell.otherMonth && !hasActivity && (
        <div className="mt-8 text-[10px] text-gray-700">No activity</div>
      )}
    </button>
  );
}

function PositionRow({ position }: { position: any }) {
  const pnl = positionPnl(position);
  return (
    <div className="grid gap-2 rounded-lg bg-gray-900 px-3 py-2 text-xs sm:grid-cols-[minmax(0,1fr)_80px_90px_90px] sm:items-center">
      <div className="min-w-0">
        <div className="truncate font-medium text-gray-200" title={position.market_title || position.market_id}>
          {position.market_title || position.market_id || "Unknown market"}
        </div>
        <div className="mt-0.5 text-[10px] capitalize text-gray-500">
          {position.platform} / {position.outcome ?? "unknown"}
        </div>
      </div>
      <div className="text-gray-400">
        {Number(position.contracts ?? 0).toFixed(2)} ct
      </div>
      <div className="text-gray-400">
        {fmtCompact(Number(position.exposure ?? 0))}
      </div>
      <div className={`font-semibold tabular-nums ${moneyColor(pnl)}`}>
        {fmtMoney(pnl)}
      </div>
    </div>
  );
}

function OrderRow({ order }: { order: any }) {
  return (
    <div className="grid gap-2 rounded-lg bg-gray-900 px-3 py-2 text-xs sm:grid-cols-[70px_80px_minmax(0,1fr)_110px_80px] sm:items-center">
      <span
        className={`w-fit rounded px-1.5 py-0.5 font-semibold ${
          order.side === "buy" ? "bg-emerald-900/70 text-emerald-300" : "bg-rose-900/70 text-rose-300"
        }`}
      >
        {order.side?.toUpperCase() ?? "ORDER"}
      </span>
      <span className="capitalize text-gray-400">{order.platform ?? "unknown"}</span>
      <span className="truncate text-gray-300" title={order.market_id}>
        {order.market_id ?? "Unknown market"}
      </span>
      <span className="text-gray-400">
        {order.requested_contracts != null ? Number(order.requested_contracts).toFixed(2) : "-"} @{" "}
        {order.price_limit != null ? `${(Number(order.price_limit) * 100).toFixed(0)}c` : "-"}
      </span>
      <span className="capitalize text-gray-500">{order.status ?? "unknown"}</span>
    </div>
  );
}

function MovementRow({ movement }: { movement: any }) {
  const amount = Number(movement.amount ?? 0);
  return (
    <div className="grid gap-2 rounded-lg bg-gray-900 px-3 py-2 text-xs sm:grid-cols-[minmax(0,1fr)_90px_90px] sm:items-center">
      <div className="min-w-0">
        <div className="truncate text-gray-300" title={movement.description || movement.movement_type}>
          {movement.description || movement.movement_type}
        </div>
        <div className="mt-0.5 text-[10px] text-gray-600">{movement.movement_type}</div>
      </div>
      <div className={`font-semibold tabular-nums ${moneyColor(amount)}`}>
        {amount >= 0 ? "+" : ""}
        {fmtMoney(amount)}
      </div>
      <div className="text-gray-500">{fmtCompact(Number(movement.balance_after ?? 0))}</div>
    </div>
  );
}

function DayDrawer({
  iso,
  orders,
  positions,
  movements,
  onClose,
}: {
  iso: string;
  orders: any[];
  positions: any[];
  movements: any[];
  onClose: () => void;
}) {
  const dayOrders = orders.filter((order) => order.submitted_at?.startsWith(iso));
  const dayPositions = positions.filter((position) => position.updated_at?.startsWith(iso));
  const dayMovements = movements.filter((movement) => movement.created_at?.startsWith(iso));
  const totalPnl = dayPositions.reduce((sum, position) => sum + positionPnl(position), 0);
  const cashFlow = dayMovements.reduce((sum, movement) => sum + Number(movement.amount ?? 0), 0);

  return (
    <section className="rounded-lg border border-gray-800 bg-gray-950 p-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h3 className="text-sm font-semibold text-white">{labelDate(iso)}</h3>
          <div className="mt-2 flex flex-wrap gap-3">
            <div>
              <div className="text-[10px] uppercase tracking-wide text-gray-500">Position P&L</div>
              <div className={`text-lg font-semibold tabular-nums ${moneyColor(totalPnl)}`}>
                {fmtMoney(totalPnl)}
              </div>
            </div>
            <div>
              <div className="text-[10px] uppercase tracking-wide text-gray-500">Wallet Cash Flow</div>
              <div className={`text-lg font-semibold tabular-nums ${moneyColor(cashFlow)}`}>
                {fmtMoney(cashFlow)}
              </div>
            </div>
          </div>
        </div>
        <button
          onClick={onClose}
          className="rounded-md border border-gray-800 px-2 py-1 text-xs text-gray-500 transition-colors hover:border-gray-700 hover:text-gray-200"
        >
          Close
        </button>
      </div>

      <div className="mt-4 grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(320px,0.6fr)]">
        <div className="space-y-4">
          <div>
            <div className="mb-2 text-[10px] font-semibold uppercase tracking-wide text-gray-500">
              Positions updated ({dayPositions.length})
            </div>
            <div className="space-y-1.5">
              {dayPositions.length > 0 ? (
                dayPositions.map((position) => <PositionRow key={position.id} position={position} />)
              ) : (
                <div className="rounded-lg bg-gray-900 px-3 py-6 text-center text-sm text-gray-600">
                  No position P&L updates on this day.
                </div>
              )}
            </div>
          </div>

          <div>
            <div className="mb-2 text-[10px] font-semibold uppercase tracking-wide text-gray-500">
              Orders ({dayOrders.length})
            </div>
            <div className="max-h-72 space-y-1.5 overflow-auto pr-1">
              {dayOrders.length > 0 ? (
                dayOrders.map((order) => <OrderRow key={order.id} order={order} />)
              ) : (
                <div className="rounded-lg bg-gray-900 px-3 py-6 text-center text-sm text-gray-600">
                  No orders on this day.
                </div>
              )}
            </div>
          </div>
        </div>

        <div>
          <div className="mb-2 text-[10px] font-semibold uppercase tracking-wide text-gray-500">
            Wallet cash ledger ({dayMovements.length})
          </div>
          <div className="max-h-[28rem] space-y-1.5 overflow-auto pr-1">
            {dayMovements.length > 0 ? (
              dayMovements.map((movement) => <MovementRow key={movement.id} movement={movement} />)
            ) : (
              <div className="rounded-lg bg-gray-900 px-3 py-6 text-center text-sm text-gray-600">
                No wallet cash movement on this day.
              </div>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}

function buildCalendarCells(viewYear: number, viewMonth: number) {
  const daysInMonth = new Date(viewYear, viewMonth + 1, 0).getDate();
  const firstDow = new Date(viewYear, viewMonth, 1).getDay();
  const prevMonthDays = new Date(viewYear, viewMonth, 0).getDate();
  const cells: CalendarCell[] = [];

  for (let i = firstDow - 1; i >= 0; i--) {
    const day = prevMonthDays - i;
    const prevMonth = viewMonth === 0 ? 11 : viewMonth - 1;
    const prevYear = viewMonth === 0 ? viewYear - 1 : viewYear;
    cells.push({ iso: isoFor(prevYear, prevMonth, day), day, otherMonth: true });
  }

  for (let day = 1; day <= daysInMonth; day++) {
    cells.push({ iso: isoFor(viewYear, viewMonth, day), day, otherMonth: false });
  }

  const trailing = (7 - (cells.length % 7)) % 7;
  for (let day = 1; day <= trailing; day++) {
    const nextMonth = viewMonth === 11 ? 0 : viewMonth + 1;
    const nextYear = viewMonth === 11 ? viewYear + 1 : viewYear;
    cells.push({ iso: isoFor(nextYear, nextMonth, day), day, otherMonth: true });
  }

  return cells;
}

export function CalendarPage() {
  const today = new Date();
  const [viewYear, setViewYear] = useState(today.getFullYear());
  const [viewMonth, setViewMonth] = useState(today.getMonth());
  const [selectedIso, setSelectedIso] = useState<string | null>(null);
  const [platformFilter, setPlatformFilter] = useState<PlatformFilter>("all");

  const { data: orders = [] } = useOrders(HISTORY_LIMIT);
  const { data: positions = [] } = usePositions(HISTORY_LIMIT);
  const { data: movements = [] } = usePaperWalletMovements(HISTORY_LIMIT);

  const filteredOrders = useMemo(
    () => platformFilter === "all" ? orders : orders.filter((order: any) => order.platform === platformFilter),
    [orders, platformFilter],
  );
  const filteredPositions = useMemo(
    () => platformFilter === "all" ? positions : positions.filter((position: any) => position.platform === platformFilter),
    [positions, platformFilter],
  );
  const filteredMovements = useMemo(
    () =>
      platformFilter === "all"
        ? movements
        : movements.filter((movement: any) => movement.payload?.platform === platformFilter),
    [movements, platformFilter],
  );

  const dailyStats = useMemo(() => {
    const map = new Map<string, DailyStat>();
    const get = (iso: string) => {
      const current = map.get(iso) ?? emptyStat();
      map.set(iso, current);
      return current;
    };

    for (const position of filteredPositions as any[]) {
      const iso = position.updated_at?.slice(0, 10);
      if (!iso) continue;
      const stat = get(iso);
      const realized = Number(position.realized_pnl ?? 0);
      const unrealized = Number(position.unrealized_pnl ?? 0);
      stat.realized += realized;
      stat.unrealized += unrealized;
      stat.pnl += realized + unrealized;
      stat.exposure += Number(position.exposure ?? 0);
      stat.positions += 1;
    }

    for (const order of filteredOrders as any[]) {
      const iso = order.submitted_at?.slice(0, 10);
      if (!iso) continue;
      get(iso).orders += 1;
    }

    for (const movement of filteredMovements as any[]) {
      const iso = movement.created_at?.slice(0, 10);
      if (!iso) continue;
      const stat = get(iso);
      stat.movements += 1;
      stat.cashFlow += Number(movement.amount ?? 0);
    }

    return map;
  }, [filteredOrders, filteredPositions, filteredMovements]);

  const monthStats = useMemo(() => {
    const prefix = `${viewYear}-${String(viewMonth + 1).padStart(2, "0")}`;
    let totalPnl = 0;
    let realized = 0;
    let unrealized = 0;
    let exposure = 0;
    let winDays = 0;
    let lossDays = 0;
    let positionDays = 0;
    let totalOrders = 0;
    let totalMovements = 0;
    let totalCashFlow = 0;
    let bestDay: number | null = null;
    let worstDay: number | null = null;
    let bestIso = "";
    let worstIso = "";

    for (const [iso, stat] of dailyStats.entries()) {
      if (!iso.startsWith(prefix)) continue;
      totalOrders += stat.orders;
      totalMovements += stat.movements;
      totalCashFlow += stat.cashFlow;

      if (stat.positions === 0) continue;
      totalPnl += stat.pnl;
      realized += stat.realized;
      unrealized += stat.unrealized;
      exposure += stat.exposure;
      positionDays += 1;
      if (stat.pnl > 0) winDays += 1;
      if (stat.pnl < 0) lossDays += 1;
      if (bestDay === null || stat.pnl > bestDay) {
        bestDay = stat.pnl;
        bestIso = iso;
      }
      if (worstDay === null || stat.pnl < worstDay) {
        worstDay = stat.pnl;
        worstIso = iso;
      }
    }

    return {
      totalPnl,
      realized,
      unrealized,
      exposure,
      winDays,
      lossDays,
      positionDays,
      totalOrders,
      totalMovements,
      totalCashFlow,
      winRate: positionDays > 0 ? (winDays / positionDays) * 100 : 0,
      bestDay,
      worstDay,
      bestIso,
      worstIso,
    };
  }, [dailyStats, viewYear, viewMonth]);

  const cells = useMemo(() => buildCalendarCells(viewYear, viewMonth), [viewYear, viewMonth]);
  const todayIso = localIsoDate(today);
  const monthLabel = new Date(viewYear, viewMonth, 1).toLocaleDateString("en-US", {
    month: "long",
    year: "numeric",
  });
  const isPossiblyLimited =
    orders.length >= HISTORY_LIMIT || positions.length >= HISTORY_LIMIT || movements.length >= HISTORY_LIMIT;

  function prevMonth() {
    if (viewMonth === 0) {
      setViewYear((year) => year - 1);
      setViewMonth(11);
    } else {
      setViewMonth((month) => month - 1);
    }
  }

  function nextMonth() {
    if (viewMonth === 11) {
      setViewYear((year) => year + 1);
      setViewMonth(0);
    } else {
      setViewMonth((month) => month + 1);
    }
  }

  function goToday() {
    setViewYear(today.getFullYear());
    setViewMonth(today.getMonth());
    setSelectedIso(todayIso);
  }

  return (
    <div className="space-y-4">
      <section className="rounded-lg border border-gray-800 bg-gray-950 p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 className="text-xl font-semibold text-gray-100">Performance Calendar</h2>
            <p className="mt-1 text-xs text-gray-500">
              Position P&L by update date, with orders and wallet cash shown separately
            </p>
          </div>
          <PlatformTabs value={platformFilter} onChange={setPlatformFilter} />
        </div>

        {isPossiblyLimited && (
          <div className="mt-3 rounded-lg border border-amber-800/50 bg-amber-950/20 px-3 py-2 text-xs text-amber-200">
            Showing up to {HISTORY_LIMIT.toLocaleString()} latest rows per dataset. Older history may need pagination.
          </div>
        )}
      </section>

      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-6">
        <StatCard
          label="Monthly P&L"
          value={fmtMoney(monthStats.totalPnl)}
          valueClass={moneyColor(monthStats.totalPnl)}
          sub={`${fmtMoney(monthStats.realized)} realized / ${fmtMoney(monthStats.unrealized)} unrealized`}
        />
        <StatCard
          label="Win Days"
          value={String(monthStats.winDays)}
          valueClass="text-emerald-300"
          sub={`${monthStats.winRate.toFixed(0)}% of P&L days`}
        />
        <StatCard
          label="Loss Days"
          value={String(monthStats.lossDays)}
          valueClass={monthStats.lossDays > 0 ? "text-rose-300" : "text-gray-100"}
          sub={`${monthStats.positionDays} P&L days`}
        />
        <StatCard
          label="Orders"
          value={monthStats.totalOrders.toLocaleString()}
          sub={`${monthStats.totalMovements} wallet events`}
        />
        <StatCard
          label="Best Day"
          value={monthStats.bestDay !== null ? fmtMoney(monthStats.bestDay) : "-"}
          valueClass="text-emerald-300"
          sub={monthStats.bestIso || undefined}
        />
        <StatCard
          label="Worst Day"
          value={monthStats.worstDay !== null ? fmtMoney(monthStats.worstDay) : "-"}
          valueClass={monthStats.worstDay !== null ? "text-rose-300" : "text-gray-100"}
          sub={monthStats.worstIso || undefined}
        />
      </div>

      <section className="rounded-lg border border-gray-800 bg-gray-950 p-4">
        <div className="mb-4 flex items-center justify-between gap-3">
          <button
            onClick={prevMonth}
            className="rounded-md border border-gray-800 px-3 py-2 text-sm text-gray-400 transition-colors hover:border-gray-700 hover:text-gray-100"
            aria-label="Previous month"
          >
            Prev
          </button>
          <div className="text-center">
            <h3 className="text-base font-semibold text-white">{monthLabel}</h3>
            <button onClick={goToday} className="mt-1 text-xs text-gray-600 transition-colors hover:text-gray-300">
              Back to today
            </button>
          </div>
          <button
            onClick={nextMonth}
            className="rounded-md border border-gray-800 px-3 py-2 text-sm text-gray-400 transition-colors hover:border-gray-700 hover:text-gray-100"
            aria-label="Next month"
          >
            Next
          </button>
        </div>

        <div className="mb-1 grid grid-cols-7 gap-1">
          {DAYS.map((day) => (
            <div key={day} className="py-1 text-center text-[10px] font-semibold uppercase tracking-wide text-gray-600">
              {day}
            </div>
          ))}
        </div>

        <div className="grid grid-cols-7 gap-1">
          {cells.map((cell) => (
            <DayCell
              key={cell.iso}
              cell={cell}
              stat={dailyStats.get(cell.iso)}
              isToday={cell.iso === todayIso}
              selected={selectedIso === cell.iso}
              onClick={() => setSelectedIso(selectedIso === cell.iso ? null : cell.iso)}
            />
          ))}
        </div>
      </section>

      {selectedIso && (
        <DayDrawer
          iso={selectedIso}
          orders={filteredOrders as any[]}
          positions={filteredPositions as any[]}
          movements={filteredMovements as any[]}
          onClose={() => setSelectedIso(null)}
        />
      )}
    </div>
  );
}
