import { ClipboardCopy, Download, Eraser, TerminalSquare, WrapText } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { cmd } from "../lib/bridge";
import { toast, useAccounts, useLogs, useT } from "../lib/store";
import type { LogLevel } from "../lib/types";
import { EmptyState, IconButton, Select, Toggle, cn } from "../components/ui";

const LEVELS: LogLevel[] = ["error", "warning", "info", "debug", "success"];

const LEVEL_STYLE: Record<LogLevel, { chip: string; label: string }> = {
  error: { chip: "border-err/35 bg-err/12 text-err", label: "console.level.error" },
  warning: { chip: "border-warn/35 bg-warn/12 text-warn", label: "console.level.warning" },
  info: { chip: "border-info/35 bg-info/12 text-info", label: "console.level.info" },
  debug: { chip: "border-dbg/35 bg-dbg/12 text-dbg", label: "console.level.debug" },
  success: { chip: "border-ok/35 bg-ok/12 text-ok", label: "console.level.success" },
};

function fmtTime(ts: number): string {
  const d = new Date(ts);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}

export function ConsolePage() {
  const t = useT();
  const { entries, clear } = useLogs();
  const accounts = useAccounts((s) => s.accounts);

  const [accountFilter, setAccountFilter] = useState<string | null>(null);
  const [levelFilter, setLevelFilter] = useState<LogLevel | null>(null);
  const [search, setSearch] = useState("");
  const [autoscroll, setAutoscroll] = useState(true);

  const listRef = useRef<HTMLDivElement>(null);
  const stickRef = useRef(true);

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    return entries.filter((e) => {
      if (accountFilter && e.account !== accountFilter) return false;
      if (levelFilter && e.level !== levelFilter) return false;
      if (q && !e.message.toLowerCase().includes(q) && !(e.account ?? "").toLowerCase().includes(q))
        return false;
      return true;
    });
  }, [entries, accountFilter, levelFilter, search]);

  // Автопрокрутка вниз (только если пользователь у низа списка)
  useEffect(() => {
    const el = listRef.current;
    if (el && autoscroll && stickRef.current) {
      el.scrollTop = el.scrollHeight;
    }
  }, [filtered.length, autoscroll]);

  const onScroll = () => {
    const el = listRef.current;
    if (!el) return;
    stickRef.current = el.scrollHeight - el.scrollTop - el.clientHeight < 60;
  };

  const exportLogs = async () => {
    const lines = filtered.map(
      (e) =>
        `${new Date(e.ts).toISOString()} [${e.level.toUpperCase().padEnd(7)}]${e.account ? ` [${e.account}]` : ""} ${e.message}`,
    );
    try {
      await cmd("export_logs", { content: lines.join("\n") + "\n" });
      toast("success", t("toast.exported"));
    } catch {
      toast("error", t("toast.logExportFailed"));
    }
  };

  const copyAll = async () => {
    const text = filtered
      .map(
        (e) =>
          `${fmtTime(e.ts)} [${e.level.toUpperCase()}]${e.account ? ` [${e.account}]` : ""} ${e.message}`,
      )
      .join("\n");
    await navigator.clipboard.writeText(text);
    toast("success", t("toast.copied"));
  };

  const accountItems = [
    { value: "__all__", label: t("console.filterAll") },
    ...accounts.map((a) => ({ value: a.username, label: a.username })),
  ];

  const levelItems = [
    { value: "__all__", label: t("console.levelAll") },
    ...LEVELS.map((l) => ({ value: l, label: t(LEVEL_STYLE[l].label) })),
  ];

  return (
    <div className="flex h-full flex-col">
      {/* Панель инструментов */}
      <div className="flex flex-wrap items-center gap-2.5 px-6 pb-2 pt-2">
        <div className="mr-auto">
          <h1 className="text-[22px] font-bold tracking-tight text-ink">{t("console.title")}</h1>
          <p className="text-[13px] text-faint">
            {t("console.subtitle")} · {t("console.lines", { n: filtered.length })}
          </p>
        </div>

        <Select
          className="w-44"
          items={accountItems}
          value={accountFilter ?? "__all__"}
          onChange={(v) => setAccountFilter(v === "__all__" ? null : v)}
        />
        <Select
          className="w-40"
          items={levelItems}
          value={levelFilter ?? "__all__"}
          onChange={(v) => setLevelFilter(v === "__all__" ? null : (v as LogLevel))}
        />

        <input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="grep…"
          className="h-9 w-40 rounded-lg border border-line bg-surface/70 px-3 text-sm text-ink placeholder:text-faint outline-none transition-all focus:border-accent/60"
        />

        <div className="flex items-center gap-2 rounded-lg border border-line bg-surface/60 px-3 py-1.5">
          <WrapText size={14} className="text-faint" />
          <span className="text-[12px] text-mut">{t("console.autoscroll")}</span>
          <Toggle checked={autoscroll} onChange={setAutoscroll} label={t("console.autoscroll")} />
        </div>

        <IconButton label={t("action.copy")} onClick={() => void copyAll()}>
          <ClipboardCopy size={16} />
        </IconButton>
        <IconButton label={t("action.export")} onClick={() => void exportLogs()}>
          <Download size={16} />
        </IconButton>
        <IconButton
          label={t("action.clear")}
          danger
          onClick={() => {
            clear();
            if (!accountFilter && !levelFilter && !search) {
              // сброс подтверждаем логом от интерфейса
              useLogs.getState().add({
                id: `ui-${Date.now()}`,
                ts: Date.now(),
                level: "debug",
                account: null,
                message: "Console cleared by user",
              });
            }
          }}
        >
          <Eraser size={16} />
        </IconButton>
      </div>

      {/* Уровни — быстрые чипы */}
      <div className="flex items-center gap-1.5 px-6 pb-2">
        <button
          onClick={() => setLevelFilter(null)}
          className={cn(
            "rounded-full border px-2.5 py-1 text-[11px] font-medium transition-all",
            levelFilter === null
              ? "border-accent/50 bg-accent/15 text-accent"
              : "border-line text-mut hover:text-ink",
          )}
        >
          {t("console.levelAll")}
        </button>
        {LEVELS.map((l) => (
          <button
            key={l}
            onClick={() => setLevelFilter(levelFilter === l ? null : l)}
            className={cn(
              "rounded-full border px-2.5 py-1 text-[11px] font-medium transition-all",
              levelFilter === l
                ? LEVEL_STYLE[l].chip
                : "border-line text-mut hover:text-ink",
            )}
          >
            {t(LEVEL_STYLE[l].label)}
          </button>
        ))}
      </div>

      {/* Логи */}
      <div
        ref={listRef}
        onScroll={onScroll}
        className="ram-scroll mx-6 mb-5 flex-1 overflow-y-auto rounded-2xl border border-line bg-[rgb(9_10_16/0.75)] shadow-card backdrop-blur-xl"
      >
        {filtered.length === 0 ? (
          <EmptyState
            icon={<TerminalSquare size={24} />}
            title={t("console.empty.title")}
            hint={t("console.empty.hint")}
          />
        ) : (
          <div className="flex flex-col divide-y divide-white/[0.04]">
            {filtered.map((e) => (
              <div key={e.id} className="flex items-start gap-3 px-4 py-[7px] hover:bg-white/[0.03]">
                <span className="shrink-0 pt-[1px] font-mono text-[11px] text-faint">
                  {fmtTime(e.ts)}
                </span>
                <span
                  className={cn(
                    "w-[86px] shrink-0 rounded-md border px-1.5 py-[1px] text-center font-mono text-[10px] font-semibold uppercase tracking-wide",
                    LEVEL_STYLE[e.level].chip,
                  )}
                >
                  {e.level.slice(0, 4)}
                </span>
                {e.account && (
                  <span className="max-w-[160px] shrink-0 truncate rounded-md bg-white/[0.06] px-1.5 py-[1px] text-[11px] text-mut">
                    {e.account}
                  </span>
                )}
                <span
                  className={cn(
                    "min-w-0 break-all text-[12.5px] leading-relaxed",
                    e.level === "error" ? "text-err/90" : e.level === "warning" ? "text-warn/90" : "text-mut",
                  )}
                >
                  {e.message}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
