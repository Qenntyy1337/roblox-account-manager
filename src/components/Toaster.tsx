import { AlertCircle, CheckCircle2, Info } from "lucide-react";
import { useToasts, type Toast } from "../lib/store";
import { cn } from "./ui";

const ICONS: Record<Toast["kind"], React.ReactNode> = {
  success: <CheckCircle2 size={16} className="text-ok" />,
  error: <AlertCircle size={16} className="text-err" />,
  info: <Info size={16} className="text-info" />,
};

export function Toaster() {
  const { toasts, dismiss } = useToasts();

  return (
    <div className="pointer-events-none fixed right-4 top-14 z-[60] flex w-80 flex-col gap-2">
      {toasts.map((t) => (
        <button
          key={t.id}
          onClick={() => dismiss(t.id)}
          className={cn(
            "pointer-events-auto flex items-center gap-2.5 rounded-xl border border-line bg-[rgb(20_22_31/0.96)] px-3.5 py-2.5 text-left text-[13px] text-ink shadow-modal backdrop-blur-xl",
            "animate-slideIn transition-transform hover:scale-[1.015]",
          )}
        >
          {ICONS[t.kind]}
          <span className="flex-1 leading-snug">{t.message}</span>
        </button>
      ))}
    </div>
  );
}
