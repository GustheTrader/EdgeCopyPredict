import { useState, useEffect } from "react";
import { Outlet, NavLink, useLocation } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import { GlobalKillSwitch } from "./GlobalKillSwitch";
import { useSignalsList, useAlerts, usePaperWalletMovements } from "@/hooks/useApi";

const PAGE_QUERY_KEYS: Record<string, string[][]> = {
  "/":               [["system-status"], ["pnl"], ["positions"], ["alerts"]],
  "/signals":        [["signals"]],
  "/masters":        [["masters"]],
  "/followers":      [["followers"], ["pnl"]],
  "/positions":      [["positions"]],
  "/orders":         [["orders"]],
  "/alerts":         [["alerts"]],
  "/trade-history":  [["orders"], ["positions"]],
  "/calendar":       [["orders"], ["paper-wallet-movements"]],
  "/raw-events":     [["raw-events"]],
  "/market-mappings":[["market-mappings"]],
  "/settings":       [["system-status"]],
};

function SidebarBadge({ count }: { count: number }) {
  if (count === 0) return null;
  return (
    <span className="ml-auto rounded-full bg-red-600 px-1.5 py-0.5 text-[10px] font-bold leading-none text-white min-w-[18px] text-center">
      {count > 99 ? "99+" : count}
    </span>
  );
}

export function Layout() {
  const qc = useQueryClient();
  const location = useLocation();
  const [collapsed, setCollapsed] = useState(() =>
    typeof window !== "undefined" ? window.innerWidth < 768 : false
  );

  const { data: signals } = useSignalsList();
  const { data: alerts } = useAlerts();
  const { data: walletMovements } = usePaperWalletMovements();

  const pendingSignals   = (signals ?? []).filter((s: any) => s.status === "new" || s.status === "pending").length;
  const unackedAlerts    = (alerts  ?? []).filter((a: any) => !a.acked).length;
  const movementCount    = (walletMovements ?? []).length;

  // Invalidate data for the page being navigated to so it always loads fresh
  useEffect(() => {
    const keys = PAGE_QUERY_KEYS[location.pathname] ?? [];
    keys.forEach((key) => qc.invalidateQueries({ queryKey: key }));
  }, [location.pathname]);

  const navItems = [
    { to: "/",              label: "Dashboard",    abbr: "D"  },
    { to: "/trade-history", label: "Trade History",abbr: "TH" },
    { to: "/calendar",      label: "Calendar",     abbr: "Ca", badge: movementCount },
    { to: "/signals",       label: "Signals",      abbr: "Si", badge: pendingSignals },
    { to: "/masters",       label: "Masters",      abbr: "M"  },
    { to: "/followers",     label: "Followers",    abbr: "F"  },
    { to: "/positions",     label: "Positions",    abbr: "Po" },
    { to: "/orders",        label: "Orders",       abbr: "Or" },
    { to: "/alerts",        label: "Alerts",       abbr: "Al", badge: unackedAlerts },
    { to: "/raw-events",    label: "Raw Events",   abbr: "R"  },
    { to: "/market-mappings", label: "Mappings",   abbr: "Ma" },
    { to: "/settings",      label: "Settings",     abbr: "Se" },
  ];

  return (
    <div className="flex h-screen bg-gray-900 text-gray-100">
      {/* Sidebar */}
      <aside
        className={`${collapsed ? "w-14" : "w-56"} shrink-0 border-r border-gray-800 bg-gray-950 flex flex-col transition-all duration-200 overflow-hidden`}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-3 py-4 border-b border-gray-800">
          {!collapsed && (
            <div>
              <h1 className="text-lg font-bold text-white leading-none">Arbitek</h1>
              <p className="text-xs text-gray-500 mt-0.5">Trade Copier</p>
            </div>
          )}
          <button
            onClick={() => setCollapsed((c) => !c)}
            title={collapsed ? "Expand sidebar" : "Collapse sidebar"}
            className={`${collapsed ? "mx-auto" : ""} rounded p-1.5 text-gray-400 hover:bg-gray-800 hover:text-gray-200 transition-colors`}
          >
            {collapsed ? (
              // chevron right
              <svg className="w-4 h-4" fill="none" stroke="currentColor" strokeWidth="2" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
              </svg>
            ) : (
              // chevron left
              <svg className="w-4 h-4" fill="none" stroke="currentColor" strokeWidth="2" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
              </svg>
            )}
          </button>
        </div>

        {/* Nav */}
        <nav className="flex-1 px-2 py-3 space-y-1">
          {navItems.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.to === "/"}
              title={collapsed ? item.label : undefined}
              className={({ isActive }) =>
                `flex items-center rounded px-2 py-2 text-sm font-medium transition-colors ${
                  isActive
                    ? "bg-gray-800 text-white"
                    : "text-gray-400 hover:bg-gray-800 hover:text-gray-200"
                }`
              }
            >
              {({ isActive }) =>
                collapsed ? (
                  <div className="relative mx-auto flex items-center justify-center w-7 h-7">
                    <span className="text-xs font-bold">{item.abbr}</span>
                    {!isActive && (item.badge ?? 0) > 0 && (
                      <span className="absolute -top-1 -right-1 rounded-full bg-red-600 px-1 text-[9px] font-bold leading-none text-white min-w-[14px] text-center py-0.5">
                        {(item.badge ?? 0) > 99 ? "99+" : item.badge}
                      </span>
                    )}
                  </div>
                ) : (
                  <>
                    <span className="flex-1">{item.label}</span>
                    {!isActive && <SidebarBadge count={item.badge ?? 0} />}
                  </>
                )
              }
            </NavLink>
          ))}
        </nav>

        {/* Kill switch */}
        <div className={`px-3 py-3 border-t border-gray-800 ${collapsed ? "flex justify-center" : ""}`}>
          {collapsed ? (
            <div
              title="Live trading toggle — expand sidebar to change"
              className="w-2 h-2 rounded-full bg-green-500"
            />
          ) : (
            <GlobalKillSwitch />
          )}
        </div>
      </aside>

      {/* Main — key forces full remount on route change, resetting all local state */}
      <main className="flex-1 overflow-auto p-4 md:p-6">
        <Outlet key={location.pathname} />
      </main>
    </div>
  );
}
