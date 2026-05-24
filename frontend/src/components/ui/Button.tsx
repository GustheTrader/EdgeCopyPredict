import { clsx } from "clsx";

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  size?: "sm" | "md";
  variant?: "primary" | "secondary" | "danger";
}

export function Button({
  children,
  size = "md",
  variant = "primary",
  className,
  ...props
}: ButtonProps) {
  return (
    <button
      className={clsx(
        "rounded font-medium",
        size === "sm" && "px-2 py-0.5 text-xs",
        size === "md" && "px-3 py-1 text-sm",
        variant === "primary" && "bg-blue-600 text-white hover:bg-blue-500",
        variant === "secondary" && "bg-gray-700 text-gray-200 hover:bg-gray-600",
        variant === "danger" && "bg-red-600 text-white hover:bg-red-500",
        className
      )}
      {...props}
    >
      {children}
    </button>
  );
}
