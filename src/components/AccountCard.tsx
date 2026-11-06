import { Pencil, Play, ShieldAlert, ShieldCheck, Square, Trash2 } from "lucide-react";
import { statusOf, useAccounts, useT } from "../lib/store";
import type { Account, InstanceStatus } from "../lib/types";
import { Button, IconButton, cn } from "./ui";

const STATUS_STYLES: Record<InstanceStatus, { dot: string; text: string; pulse?: boolean }> = {
  offline: { dot: "bg-faint", text: "text-faint" },
  launching: { dot: "bg-warn", text: "text-warn", pulse: true },
  online: { dot: "bg-info", text: "text-info" },
  ingame: { dot: "bg-ok", text: "text-ok" },
};

function avatarHue(name: string): number {
  let h = 0;
  for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) | 0;
  return Math.abs(h) % 360;
}

export function StatusChip({ status }: { status: InstanceStatus }) {
  const t = useT();
  const s = STATUS_STYLES[status];
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full border border-line bg-surface/80 px-2.5 py-1 text-[11px] font-medium",
        s.text,
      )}
    >
      <span className={cn("h-1.5 w-1.5 rounded-full", s.dot, s.pulse && "animate-pulseDot")} />
      {t(`status.${status}`)}
    </span>
  );
}

export function AccountCard({
  account,
  onEdit,
  onDelete,
  onTerminateAsk,
}: {
  account: Account;
  onEdit: () => void;
  onDelete: () => void;
  onTerminateAsk: () => void;
}) {
  const t = useT();
  const { instances, presence, launch } = useAccounts();
  const status = statusOf(account, instances, presence);
  const inst = instances[account.id];
  const running = status !== "offline";
  const hue = avatarHue(account.username);

  return (
    <div
      className={cn(
        "group flex flex-col gap-3.5 rounded-2xl border border-line bg-surface/60 p-4 shadow-card backdrop-blur-xl",
        "transition-all duration-200 hover:-translate-y-0.5 hover:border-accent/45 hover:shadow-glowSm",
        running && "border-accent/30",
      )}
    >
      <div className="flex items-start gap-3">
        {account.avatar_url ? (
          <img
            src={account.avatar_url}
            alt=""
            className="h-11 w-11 shrink-0 rounded-xl border border-line object-cover"
          />
        ) : (
          <div
            className="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl text-[17px] font-bold text-white shadow-card"
            style={{
              background: `linear-gradient(135deg, hsl(${hue} 62% 46%), hsl(${(hue + 46) % 360} 62% 38%))`,
            }}
          >
            {account.username.charAt(0).toUpperCase()}
          </div>
        )}

        <div className="min-w-0 flex-1">
          <div className="truncate text-[15px] font-semibold text-ink">{account.username}</div>
          <div className="truncate text-xs text-faint">
            {account.note || t("accounts.id", { id: account.user_id ?? "—" })}
          </div>
        </div>

        <div className="flex items-center gap-1 opacity-80 transition-opacity group-hover:opacity-100">
          <IconButton label={t("action.edit")} onClick={onEdit}>
            <Pencil size={15} />
          </IconButton>
          <IconButton label={t("action.delete")} danger onClick={onDelete}>
            <Trash2 size={15} />
          </IconButton>
        </div>
      </div>

      <div className="flex items-center gap-2">
        <StatusChip status={status} />
        <span
          className={cn(
            "inline-flex items-center gap-1.5 rounded-full border border-line px-2.5 py-1 text-[11px]",
            account.cookie_ok ? "text-ok" : "text-faint",
          )}
        >
          {account.cookie_ok ? <ShieldCheck size={12} /> : <ShieldAlert size={12} />}
          {account.cookie_ok ? t("accounts.cookieBadge.ok") : t("accounts.cookieBadge.bad")}
        </span>
        {inst && (
          <span className="ml-auto font-mono text-[11px] text-faint">PID {inst.pid}</span>
        )}
      </div>

      <div className="flex items-center gap-2">
        {running ? (
          <Button
            variant="danger"
            size="sm"
            icon={<Square size={13} />}
            onClick={onTerminateAsk}
          >
            {t("action.terminate")}
          </Button>
        ) : (
          <Button variant="primary" size="sm" icon={<Play size={13} />} onClick={() => void launch(account.id)}>
            {t("action.launch")}
          </Button>
        )}
      </div>
    </div>
  );
}
