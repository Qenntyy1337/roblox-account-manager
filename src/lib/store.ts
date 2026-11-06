import { create } from "zustand";
import { cmd, genId, listenEvent } from "./bridge";
import { makeT, type TFunc } from "./i18n";
import { showNotification } from "./notify";
import { applyAccent } from "./theme";
import { playClick, playCrash, playError, playSuccess, setSoundEnabled } from "./sound";
import {
  defaultSettings,
  type Account,
  type AccountInput,
  type InstanceInfo,
  type LogEntry,
  type PresenceMap,
  type Settings,
} from "./types";

/* ------------------------------------------------------------------ */
/* Тосты                                                               */
/* ------------------------------------------------------------------ */

export type ToastKind = "success" | "error" | "info";
export interface Toast {
  id: string;
  kind: ToastKind;
  message: string;
}

interface ToastState {
  toasts: Toast[];
  push: (kind: ToastKind, message: string) => void;
  dismiss: (id: string) => void;
}

export const useToasts = create<ToastState>((set, get) => ({
  toasts: [],
  push(kind, message) {
    const id = genId();
    set((s) => ({ toasts: [...s.toasts.slice(-4), { id, kind, message }] }));
    window.setTimeout(() => get().dismiss(id), 4200);
  },
  dismiss(id) {
    set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) }));
  },
}));

export function toast(kind: ToastKind, message: string): void {
  useToasts.getState().push(kind, message);
  if (kind === "error") playError();
  else if (kind === "success") playSuccess();
  else playClick();
}

/* ------------------------------------------------------------------ */
/* Настройки                                                           */
/* ------------------------------------------------------------------ */

interface SettingsState {
  settings: Settings;
  loaded: boolean;
  t: TFunc;
  hydrate: (s: Settings) => void;
  patch: (p: Partial<Settings>) => Promise<void>;
}

let saveTimer: number | undefined;

export const useSettings = create<SettingsState>((set, get) => ({
  settings: { ...defaultSettings },
  loaded: false,
  t: makeT(defaultSettings.language),
  hydrate(s) {
    set({ settings: s, loaded: true, t: makeT(s.language) });
    applyAccent(s.accent);
    setSoundEnabled(s.soundEnabled);
    document.documentElement.lang = s.language;
  },
  async patch(p) {
    const next = { ...get().settings, ...p };
    set({ settings: next });
    if (p.accent !== undefined) applyAccent(p.accent);
    if (p.soundEnabled !== undefined) setSoundEnabled(p.soundEnabled);
    if (p.language !== undefined) {
      set({ t: makeT(p.language) });
      document.documentElement.lang = p.language;
    }
    window.clearTimeout(saveTimer);
    saveTimer = window.setTimeout(() => {
      void cmd("save_settings", { settings: get().settings });
    }, 250);
  },
}));

export const useT = (): TFunc => useSettings((s) => s.t);

/** Помощники первичной загрузки для App */
export function hydrateSettings(s: Settings): void {
  useSettings.getState().hydrate(s);
}

export function hydrateAccounts(accounts: Account[], instances: InstanceInfo[]): void {
  useAccounts.getState().hydrate(accounts, instances);
}

/* ------------------------------------------------------------------ */
/* Аккаунты и инстансы                                                 */
/* ------------------------------------------------------------------ */

interface AccountsState {
  accounts: Account[];
  instances: Record<string, InstanceInfo>;
  presence: PresenceMap;
  loaded: boolean;
  hydrate: (accounts: Account[], instances: InstanceInfo[]) => void;
  setInstances: (list: InstanceInfo[]) => void;
  add: (input: AccountInput) => Promise<Account>;
  update: (id: string, input: Omit<AccountInput, "cookie"> & { cookie?: string }) => Promise<void>;
  remove: (id: string) => Promise<void>;
  launch: (id: string) => Promise<void>;
  launchAll: () => Promise<void>;
  terminate: (id: string) => Promise<void>;
  closeAll: () => Promise<void>;
  refreshPresence: () => Promise<void>;
}

const sleep = (ms: number) => new Promise((r) => window.setTimeout(r, ms));

export const useAccounts = create<AccountsState>((set, get) => ({
  accounts: [],
  instances: {},
  presence: {},
  loaded: false,
  hydrate(accounts, list) {
    const instances: Record<string, InstanceInfo> = {};
    for (const i of list) instances[i.account_id] = i;
    set({ accounts, instances, loaded: true });
  },
  setInstances(list) {
    const map: Record<string, InstanceInfo> = {};
    for (const i of list) map[i.account_id] = i;
    set({ instances: map });
  },
  async add(input) {
    const acc = await cmd<Account>("add_account", { input });
    set((s) => ({ accounts: [...s.accounts, acc] }));
    return acc;
  },
  async update(id, input) {
    const payload: Record<string, unknown> = { username: input.username, note: input.note };
    if (typeof input.cookie === "string" && input.cookie.trim().length > 0) {
      payload.cookie = input.cookie;
    }
    const updated = await cmd<Account>("update_account", { id, input: payload });
    set((s) => ({ accounts: s.accounts.map((a) => (a.id === id ? updated : a)) }));
  },
  async remove(id) {
    await cmd("delete_account", { id });
    set((s) => {
      const instances = { ...s.instances };
      delete instances[id];
      return { accounts: s.accounts.filter((a) => a.id !== id), instances };
    });
  },
  async launch(id) {
    const acc = get().accounts.find((a) => a.id === id);
    if (!acc) return;
    try {
      const res = await cmd<{ pid: number; mode: string }>("launch_account", { id });
      const st = useSettings.getState();
      if (res.mode === "autologin") {
        toast("success", st.t("toast.clientLaunchedAuto"));
      } else {
        toast("success", st.t("toast.clientLaunched", { pid: res.pid }));
      }
    } catch (e) {
      toast("error", useSettings.getState().t("toast.launchFailed", { err: String(e) }));
    }
  },
  async launchAll() {
    const { accounts, instances } = get();
    const delay = useSettings.getState().settings.launchDelayMs;
    let first = true;
    for (const acc of accounts) {
      if (instances[acc.id]) continue;
      if (!first) await sleep(Math.max(200, delay));
      first = false;
      await get().launch(acc.id);
    }
  },
  async terminate(id) {
    await cmd("terminate_instance", { id });
    set((s) => {
      const instances = { ...s.instances };
      delete instances[id];
      return { instances };
    });
  },
  async closeAll() {
    await cmd("close_all", {});
    set({ instances: {} });
  },
  async refreshPresence() {
    try {
      const map = await cmd<PresenceMap>("get_presence", {});
      set({ presence: map });
    } catch {
      /* сеть недоступна — пропускаем */
    }
  },
}));

/** Итоговый статус аккаунта: локальный инстанс важнее presence */
export function statusOf(
  account: Account,
  instances: Record<string, InstanceInfo>,
  presence: PresenceMap,
): InstanceInfo["status"] {
  const inst = instances[account.id];
  if (inst) return inst.status;
  if (account.user_id != null) {
    const p = presence[account.user_id];
    if (p === 2 || p === 3) return "ingame";
    if (p === 1) return "online";
  }
  return "offline";
}

/* ------------------------------------------------------------------ */
/* Логи                                                                */
/* ------------------------------------------------------------------ */

interface LogsState {
  entries: LogEntry[];
  add: (e: LogEntry) => void;
  clear: () => void;
}

export const useLogs = create<LogsState>((set) => ({
  entries: [],
  add(e) {
    set((s) => ({ entries: [...s.entries.slice(-2999), e] }));
  },
  clear() {
    set({ entries: [] });
  },
}));

/** Уведомления Windows по ключевым событиям (даже при свёрнутом окне) */
function maybeNotify(e: LogEntry): void {
  const st = useSettings.getState();
  if (!st.settings.winNotifications) return;
  const name = e.account ?? "Roblox";
  const msg = e.message;
  if (e.level === "error" && msg.includes("CRASHED")) {
    void showNotification(st.t("app.title"), st.t("notif.crashed", { name }));
  } else if (e.level === "success" && msg.includes("Client started")) {
    void showNotification(st.t("app.title"), st.t("notif.started", { name }));
  } else if (e.level === "success" && msg.startsWith("Joined game")) {
    void showNotification(st.t("app.title"), st.t("notif.joined", { name }));
  }
}

/** Подписка на события бэкенда — вызывается один раз при старте приложения */
export function initEventBridge(): void {
  listenEvent("log", (payload) => {
    const e = payload as LogEntry;
    useLogs.getState().add(e);
    // Звуки событий: ошибка/краш — громкий сигнал, вход в игру — радостный
    if (e.level === "error") {
      if (e.message.toLowerCase().includes("crashed")) playCrash();
      else playError();
    } else if (e.level === "success" && e.message.startsWith("Joined game")) {
      playSuccess();
    }
    maybeNotify(e);
  });
  listenEvent("instances", (payload) => {
    useAccounts.getState().setInstances(payload as InstanceInfo[]);
  });
}
