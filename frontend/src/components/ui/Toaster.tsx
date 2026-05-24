import { useToastStore, ToastItem } from "@/stores/toastStore";

const STYLES: Record<ToastItem["type"], { bar: string; icon: string; label: string }> = {
  success: { bar: "bg-green-500",  icon: "✓", label: "text-green-400" },
  error:   { bar: "bg-red-500",    icon: "✕", label: "text-red-400"   },
  warning: { bar: "bg-yellow-500", icon: "!", label: "text-yellow-400" },
  info:    { bar: "bg-blue-500",   icon: "i", label: "text-blue-400"  },
};

function Toast({ toast }: { toast: ToastItem }) {
  const remove = useToastStore((s) => s.remove);
  const s = STYLES[toast.type];

  return (
    <div className="flex items-start gap-3 rounded-lg border border-gray-700 bg-gray-900 shadow-xl w-80 overflow-hidden">
      <div className={`w-1 self-stretch shrink-0 ${s.bar}`} />
      <span className={`mt-3 text-sm font-bold ${s.label}`}>{s.icon}</span>
      <p className="flex-1 py-3 pr-1 text-sm text-gray-200 leading-snug">{toast.message}</p>
      <button
        onClick={() => remove(toast.id)}
        className="mt-2.5 mr-2 text-gray-500 hover:text-gray-300 text-lg leading-none"
      >
        ×
      </button>
    </div>
  );
}

export function Toaster() {
  const toasts = useToastStore((s) => s.toasts);
  if (toasts.length === 0) return null;

  return (
    <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2">
      {toasts.map((t) => (
        <Toast key={t.id} toast={t} />
      ))}
    </div>
  );
}
