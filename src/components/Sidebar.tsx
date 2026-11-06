import { TerminalSquare, Users, SlidersHorizontal } from "lucide-react";
import { useT } from "../lib/store";
import { LogoMark } from "./TitleBar";
import { cn } from "./ui";

export type PageId = "accounts" | "console" | "settings";

const NAV: { id: PageId; labelKey: string; icon: React.ReactNode }[] = [
  { id: "accounts", labelKey: "nav.accounts", icon: <Users size={17} /> },
  { id: "console", labelKey: "nav.console", icon: <TerminalSquare size={17} /> },
  { id: "settings", labelKey: "nav.settings", icon: <SlidersHorizontal size={17} /> },
];

export function Sidebar({
  page,
  onNavigate,
  version,
}: {
  page: PageId;
  onNavigate: (p: PageId) => void;
  version: string;
}) {
  const t = useT();

  return (
    <aside className="flex w-[228px] shrink-0 flex-col border-r border-line/70 bg-panel/50 backdrop-blur-2xl">
      <div className="flex items-center gap-2.5 px-5 pb-4 pt-6">
        <LogoMark size={30} />
        <div className="leading-tight">
          <div className="text-[13px] font-bold text-ink">Roblox</div>
          <div className="text-[11px] font-medium text-faint">Account Manager</div>
        </div>
      </div>

      <nav className="flex flex-col gap-1 px-3 pt-2">
        {NAV.map((item) => {
          const active = page === item.id;
          return (
            <button
              key={item.id}
              onClick={() => onNavigate(item.id)}
              className={cn(
                "group relative flex items-center gap-3 rounded-xl px-3 py-2.5 text-sm font-medium transition-all duration-150",
                active
                  ? "bg-accent/12 text-accent"
                  : "text-mut hover:bg-white/[0.05] hover:text-ink",
              )}
            >
              <span
                className={cn(
                  "absolute left-0 top-1/2 h-5 w-[3px] -translate-y-1/2 rounded-full transition-all duration-200",
                  active ? "bg-accent opacity-100 shadow-glowSm" : "opacity-0",
                )}
              />
              {item.icon}
              {t(item.labelKey)}
            </button>
          );
        })}
      </nav>

      <div className="mt-auto px-5 pb-5">
        <div className="border-t border-line/60 pt-3 text-[11px] text-faint">
          v{version}
        </div>
      </div>
    </aside>
  );
}
