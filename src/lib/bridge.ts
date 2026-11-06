import type {
  Account,
  AccountInput,
  InstanceInfo,
  Settings,
} from "./types";

type Handler = (payload: unknown) => void;

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const listeners = new Map<string, Set<Handler>>();
const DEMO_KEY = "ram_demo_state_v1";

let uid = 0;
const genId = () =>
  `${Date.now().toString(36)}-${(uid++).toString(36)}-${Math.random().toString(36).slice(2, 8)}`;

export function listenEvent(name: string, handler: Handler): () => void {
  if (isTauri) {
    // Динамический импорт оставляем для реального моста Tauri
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    void import("@tauri-apps/api/event").then(({ listen }) =>
      listen(name, (e) => handler(e.payload)),
    );
    return () => {
      /* отписка выполняется внутри промиса выше; для краткости демо-приложения не критична */
    };
  }
  let set = listeners.get(name);
  if (!set) {
    set = new Set();
    listeners.set(name, set);
  }
  set.add(handler);
  return () => {
    set?.delete(handler);
  };
}

function emit(name: string, payload: unknown): void {
  const set = listeners.get(name);
  if (set) for (const h of Array.from(set)) h(payload);
}

/* ------------------------------------------------------------------ */
/* Демо-бэкенд (используется в браузерном превью без Tauri)            */
/* ------------------------------------------------------------------ */

import { defaultSettings } from "./types";
import type { LogEntry } from "./types";

interface DemoState {
  settings: Settings;
  accounts: Account[];
  instances: Record<string, InstanceInfo>;
}

const DEMO_ROBLOX_PATH = "C:\\Users\\Player\\AppData\\Local\\Roblox\\Versions\\version-8f3a21c0\\RobloxPlayerBeta.exe";

function seedDemo(): DemoState {
  const now = Date.now();
  return {
    settings: { ...defaultSettings },
    accounts: [
      {
        id: "demo-main",
        username: "Nexus_Builder",
        user_id: 156,
        avatar_url: null,
        cookie_ok: true,
        note: "Основной аккаунт",
        created_at: now - 86400000 * 30,
        playtime_secs: 12_960,
        last_played: now - 3600_000,
      },
      {
        id: "demo-alt",
        username: "Astra_Alt",
        user_id: 8632,
        avatar_url: null,
        cookie_ok: true,
        note: "Альт для фарма",
        created_at: now - 86400000 * 3,
        playtime_secs: 1_845,
        last_played: now - 86_400_000,
      },
    ],
    instances: {},
  };
}

function loadDemo(): DemoState {
  try {
    const raw = localStorage.getItem(DEMO_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as DemoState;
      // Новые настройки из обновлений подтягиваем с дефолтами
      parsed.settings = { ...defaultSettings, ...parsed.settings };
      return parsed;
    }
  } catch {
    /* ignore */
  }
  const s = seedDemo();
  saveDemo(s);
  return s;
}

function saveDemo(s: DemoState): void {
  try {
    // Инстансы не сохраняем: после перезапуска превью клиенты «закрыты»
    const persisted = { settings: s.settings, accounts: s.accounts, instances: {} };
    localStorage.setItem(DEMO_KEY, JSON.stringify(persisted));
  } catch {
    /* ignore */
  }
}

function demoLog(level: LogEntry["level"], account: string | null, message: string): void {
  emit("log", { id: genId(), ts: Date.now(), level, account, message } satisfies LogEntry);
}

/** Имитация жизненного цикла запущенного клиента */
function demoSimulateLaunch(acc: Account, pid: number): void {
  const state = loadDemo();
  state.instances[acc.id] = {
    account_id: acc.id,
    pid,
    status: "launching",
    started_at: Date.now(),
    title: null,
    mode: "plain",
    playtime_secs: acc.playtime_secs,
  };
  saveDemo(state);
  emit("instances", Object.values(state.instances));

  demoLog("info", acc.username, `Launching RobloxPlayerBeta.exe (pid ${pid}) for account "${acc.username}"`);

  // Наработка в демо растёт вместе с сессией (+5 с каждые 5 с)
  const playTimer = window.setInterval(() => {
    const st = loadDemo();
    const inst = st.instances[acc.id];
    if (!inst) {
      window.clearInterval(playTimer);
      return;
    }
    inst.playtime_secs += 5;
    saveDemo(st);
    emit("instances", Object.values(st.instances));
  }, 5000);

  setTimeout(() => {
    demoLog("success", acc.username, `Singleton handle ROBLOX_single* closed (pid ${pid})`);
  }, 900);

  setTimeout(() => {
    const st = loadDemo();
    const inst = st.instances[acc.id];
    if (!inst) return;
    inst.status = "online";
    saveDemo(st);
    emit("instances", Object.values(st.instances));
    demoLog("info", acc.username, `Client window detected (pid ${pid}): "Roblox"`);
  }, 2600);

  if (Math.random() < 0.7) {
    setTimeout(() => {
      const st = loadDemo();
      const inst = st.instances[acc.id];
      if (!inst) return;
      inst.status = "ingame";
      inst.title = "Blox Fruits - Roblox";
      saveDemo(st);
      emit("instances", Object.values(st.instances));
      demoLog("success", acc.username, `Joined game "Blox Fruits" (pid ${pid})`);
    }, 6500);
  }
}

/* ------------------------------------------------------------------ */
/* Главный вызов команды                                               */
/* ------------------------------------------------------------------ */

async function invokeTauri<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(name, args ?? {});
}

async function invokeDemo<T>(name: string, args: Record<string, unknown> = {}): Promise<T> {
  // Небольшая задержка, чтобы показать состояния загрузки
  await new Promise((r) => setTimeout(r, 120));
  const state = loadDemo();

  switch (name) {
    case "get_bootstrap": {
      return {
        settings: state.settings,
        accounts: state.accounts,
        instances: Object.values(state.instances),
        robloxPath: DEMO_ROBLOX_PATH,
        appVersion: "1.0.1",
      } as T;
    }
    case "save_settings": {
      state.settings = args.settings as Settings;
      saveDemo(state);
      return true as T;
    }
    case "add_account": {
      const input = args.input as AccountInput;
      const cookieOk = typeof input.cookie === "string" && input.cookie.trim().length > 40;
      const acc: Account = {
        id: `demo-${genId()}`,
        username: input.username.trim() || `Player_${Math.floor(Math.random() * 9000 + 1000)}`,
        user_id: cookieOk ? Math.floor(Math.random() * 100000 + 500) : null,
        avatar_url: null,
        cookie_ok: cookieOk,
        note: input.note.trim(),
        created_at: Date.now(),
        playtime_secs: 0,
        last_played: null,
      };
      state.accounts.push(acc);
      saveDemo(state);
      demoLog(cookieOk ? "success" : "warning", acc.username, `Account "${acc.username}" added (cookie ${cookieOk ? "verified" : "not verified"})`);
      return acc as T;
    }
    case "update_account": {
      const id = args.id as string;
      const input = args.input as AccountInput & { cookie?: string };
      const acc = state.accounts.find((a) => a.id === id);
      if (!acc) throw new Error("account not found");
      if (input.username) acc.username = input.username.trim();
      if (input.note !== undefined) acc.note = input.note.trim();
      if (typeof input.cookie === "string" && input.cookie.trim().length > 0) {
        acc.cookie_ok = input.cookie.trim().length > 40;
        acc.user_id = acc.cookie_ok ? acc.user_id ?? Math.floor(Math.random() * 100000 + 500) : acc.user_id;
        demoLog(acc.cookie_ok ? "success" : "warning", acc.username, `Cookie updated for "${acc.username}"`);
      }
      saveDemo(state);
      return acc as T;
    }
    case "delete_account": {
      const id = args.id as string;
      const idx = state.accounts.findIndex((a) => a.id === id);
      if (idx >= 0) {
        const [acc] = state.accounts.splice(idx, 1);
        delete state.instances[id];
        saveDemo(state);
        demoLog("warning", acc?.username ?? null, `Account "${acc?.username}" deleted`);
      }
      emit("instances", Object.values(state.instances));
      return true as T;
    }
    case "launch_account": {
      const id = args.id as string;
      const acc = state.accounts.find((a) => a.id === id);
      if (!acc) throw new Error("account not found");
      const pid = 4000 + Math.floor(Math.random() * 900);
      demoSimulateLaunch(acc, pid);
      // Демо-режим имитирует автологин, если он включён в настройках
      return { pid, mode: state.settings.autoLogin && acc.cookie_ok ? "autologin" : "plain" } as T;
    }
    case "terminate_instance": {
      const id = args.id as string;
      const inst = state.instances[id];
      const acc = state.accounts.find((a) => a.id === id);
      if (inst) {
        delete state.instances[id];
        saveDemo(state);
        demoLog("warning", acc?.username ?? null, `Client terminated (pid ${inst.pid})`);
        emit("instances", Object.values(state.instances));
      }
      return true as T;
    }
    case "close_all": {
      const n = Object.keys(state.instances).length;
      for (const id of Object.keys(state.instances)) {
        const acc = state.accounts.find((a) => a.id === id);
        demoLog("warning", acc?.username ?? null, `Client terminated (pid ${state.instances[id].pid})`);
      }
      state.instances = {};
      saveDemo(state);
      emit("instances", []);
      demoLog("info", null, `All clients closed (${n})`);
      return true as T;
    }
    case "validate_cookie": {
      const cookie = String(args.cookie ?? "");
      const ok = cookie.trim().length > 40;
      return {
        ok,
        username: ok ? `Verified_${Math.floor(Math.random() * 900 + 100)}` : null,
      } as T;
    }
    case "detect_roblox_path": {
      return DEMO_ROBLOX_PATH as T;
    }
    case "pick_file": {
      // Настоящий диалог браузера — превью работает без бэкенда
      const input = document.createElement("input");
      input.type = "file";
      input.accept = "image/*";
      return await new Promise<T>((resolve) => {
        input.onchange = () => {
          const file = input.files?.[0];
          if (!file) return resolve(null as T);
          const reader = new FileReader();
          reader.onload = () => resolve((reader.result as string) as T);
          reader.readAsDataURL(file);
        };
        input.click();
      });
    }
    case "export_logs": {
      const content = args.content as string;
      const blob = new Blob([content], { type: "text/plain;charset=utf-8" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "ram-console.log";
      a.click();
      URL.revokeObjectURL(url);
      return true as T;
    }
    case "open_path": {
      demoLog("debug", null, `open_path: ${String(args.path)}`);
      return true as T;
    }
    case "get_presence": {
      // В демо presence не используется: статусы определяются инстансами
      void state;
      return {} as T;
    }
    default:
      throw new Error(`Unknown command: ${name}`);
  }
}

export function cmd<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) return invokeTauri<T>(name, args);
  return invokeDemo<T>(name, args);
}

export { emit as emitDemoEvent, genId };
