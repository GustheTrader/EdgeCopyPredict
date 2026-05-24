import { clsx } from "clsx";

interface BadgeProps {
  children: React.ReactNode;
  color?: "red" | "yellow" | "green" | "blue";
}

export function Badge({ children, color = "blue" }: BadgeProps) {
  const map = {
    red: "bg-red-600 text-white",
    yellow: "bg-yellow-500 text-black",
    green: "bg-green-600 text-white",
    blue: "bg-blue-600 text-white",
  };
  return (
    <span className={clsx("rounded px-2 py-0.5 text-xs font-medium", map[color])}>
      {children}
    </span>
  );
}
