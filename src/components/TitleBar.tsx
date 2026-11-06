import { Minus, Square, X } from "lucide-react";
import { isTauri } from "../lib/bridge";
import { useT } from "../lib/store";
import { IconButton, cn } from "./ui";

/** Логотип: два перекрывающихся окна = мульти-инстанс */
export function LogoMark({ size = 26 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 26 26" aria-hidden>
      <defs>
        <linearGradient id="ram-logo-g" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" style={{ stopColor: "rgb(var(--accent-rgb))" }} />
          <stop offset="1" style={{ stopColor: "rgb(var(--accent-rgb) / 0.55)" }} />
        </linearGradient>
      </defs>
      <rect x="0.5" y="0.5" width="25" height="25" rx="7.5" fill="url(#ram-logo-g)" />
      <rect
        x="6"
        y="7.5"
        width="10.5"
        height="8.5"
        rx="1.8"
        fill="none"
        stroke="rgba(255,255,255,0.92)"
        strokeWidth="1.5"
      />
      <rect
        x="9.5"
        y="10.5"
        width="10.5"
        height="8.5"
        rx="1.8"
        fill="rgb(8 9 14 / 0.35)"
        stroke="rgba(255,255,255,0.55)"
        strokeWidth="1.5"
      />
    </svg>
  );
}

interface WinApi {
  minimize(): Promise<void>;
  toggleMaximize(): Promise<void>;
  close(): Promise<void>;
  startDragging(): Promise<void>;
}

async function getWin(): Promise<WinApi | null> {
  if (!isTauri) return null;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow();
}

export function TitleBar({ demo }: { demo: boolean }) {
  const t = useT();

  return (
    <header
      className="flex h-11 shrink-0 items-center justify-between pl-4 select-none"
      onMouseDown={(e) => {
        if ((e.target as HTMLElement).closest("button")) return;
        void getWin()?.then((w) => w?.startDragging());
      }}
      onDoubleClick={() => void getWin()?.then((w) => w?.toggleMaximize())}
    >
      <div className="flex items-center gap-2.5 pointer-events-none">
        <LogoMark />
        <span className="text-[13px] font-semibold tracking-wide text-ink">
          {t("app.title")}
        </span>
        {demo && (
          <span className="ml-1 rounded-md border border-warn/30 bg-warn/10 px-2 py-0.5 text-[11px] font-medium text-warn">
            {t("demo.badge")}
          </span>
        )}
      </div>

      <div className="flex items-center">
        <IconButton
          label="minimize"
          className={cn("h-11 w-12 rounded-none", !isTauri && "opacity-30 pointer-events-none")}
          onClick={() => void getWin()?.then((w) => w?.minimize())}
        >
          <Minus size={15} />
        </IconButton>
        <IconButton
          label="maximize"
          className={cn("h-11 w-12 rounded-none", !isTauri && "opacity-30 pointer-events-none")}
          onClick={() => void getWin()?.then((w) => w?.toggleMaximize())}
        >
          <Square size={13} />
        </IconButton>
        <IconButton
          label="close"
          danger
          className={cn(
            "h-11 w-12 rounded-none hover:!bg-[#e81123] hover:!text-white",
            !isTauri && "opacity-30 pointer-events-none",
          )}
          onClick={() => void getWin()?.then((w) => w?.close())}
        >
          <X size={16} />
        </IconButton>
      </div>
    </header>
  );
}
