export type Lang = "en" | "ru" | "uk";

export type LogLevel = "error" | "warning" | "info" | "debug" | "success";

export type InstanceStatus = "offline" | "launching" | "online" | "ingame";

export interface Account {
  id: string;
  /** Отображаемое имя (валидированное из API либо введённое вручную) */
  username: string;
  user_id: number | null;
  avatar_url: string | null;
  /** Cookie проверена и признана действительной */
  cookie_ok: boolean;
  note: string;
  created_at: number;
  /** Суммарная наработка за все сессии, секунды */
  playtime_secs: number;
  /** Конец последней сессии, unix ms */
  last_played: number | null;
}

export interface AccountInput {
  username: string;
  cookie: string;
  note: string;
}

export interface InstanceInfo {
  account_id: string;
  pid: number;
  status: InstanceStatus;
  started_at: number;
  title: string | null;
  /** Как запущен клиент: autologin | plain */
  mode: string | null;
  /** Суммарная наработка (сохранённая + текущая сессия), секунды */
  playtime_secs: number;
}

export interface LogEntry {
  id: string;
  ts: number;
  level: LogLevel;
  account: string | null;
  message: string;
}

export interface Settings {
  accent: string;
  language: Lang;
  bgImage: string | null;
  /** 0..100 — непрозрачность фонового изображения */
  bgOpacity: number;
  /** 0..40 px — размытие фонового изображения */
  bgBlur: number;
  /** Пауза между последовательными запусками, мс */
  launchDelayMs: number;
  /** Изолировать данные клиента по аккаунту (подмена LOCALAPPDATA) */
  isolateData: boolean;
  /** Закрывать синглтон-хэндлы Roblox (мульти-инстанс) */
  closeSingleton: boolean;
  /** Автологин: клиент запускается сразу с нужным аккаунтом */
  autoLogin: boolean;
  /** Звуковые эффекты интерфейса (клики, ошибки, краши) */
  soundEnabled: boolean;
  /** Уведомления Windows (краш, запуск, вход в игру) */
  winNotifications: boolean;
  robloxPath: string | null;
}

export interface Bootstrap {
  settings: Settings;
  accounts: Account[];
  instances: InstanceInfo[];
  robloxPath: string | null;
  appVersion: string;
}

/** Roblox presence: 0 — офлайн, 1 — на сайте, 2 — в игре, 3 — в Studio */
export type PresenceMap = Record<number, number>;

export const defaultSettings: Settings = {
  accent: "#8b5cf6",
  language: "ru",
  bgImage: null,
  bgOpacity: 55,
  bgBlur: 14,
  launchDelayMs: 900,
  isolateData: true,
  closeSingleton: true,
  autoLogin: true,
  soundEnabled: true,
  winNotifications: true,
  robloxPath: null,
};
