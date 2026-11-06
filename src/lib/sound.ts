/**
 * Звуковые эффекты интерфейса — синтезируются через Web Audio API
 * на лету, без внешних файлов. Уважает настройку soundEnabled.
 */

let ctx: AudioContext | null = null;
let enabled = true;
const volume = 0.16;

export function setSoundEnabled(v: boolean): void {
  enabled = v;
}

function audio(): AudioContext | null {
  if (!enabled) return null;
  try {
    if (!ctx) ctx = new AudioContext();
    if (ctx.state === "suspended") void ctx.resume();
    return ctx;
  } catch {
    return null;
  }
}

interface ToneOpts {
  freq: number;
  dur: number;
  type?: OscillatorType;
  vol?: number;
  delay?: number;
  slideTo?: number;
}

function tone({ freq, dur, type = "sine", vol = 1, delay = 0, slideTo }: ToneOpts): void {
  const c = audio();
  if (!c) return;
  const t0 = c.currentTime + delay;
  const osc = c.createOscillator();
  const gain = c.createGain();
  osc.type = type;
  osc.frequency.setValueAtTime(freq, t0);
  if (slideTo && slideTo > 0) {
    osc.frequency.exponentialRampToValueAtTime(slideTo, t0 + dur);
  }
  gain.gain.setValueAtTime(0.0001, t0);
  gain.gain.exponentialRampToValueAtTime(Math.max(0.0002, volume * vol), t0 + 0.012);
  gain.gain.exponentialRampToValueAtTime(0.0001, t0 + dur);
  osc.connect(gain);
  gain.connect(c.destination);
  osc.start(t0);
  osc.stop(t0 + dur + 0.05);
}

/** Короткий тихий клик — все кнопки интерфейса */
export function playClick(): void {
  tone({ freq: 1900, dur: 0.05, vol: 0.35 });
}

/** Успех: приятное двухнотное созвучие */
export function playSuccess(): void {
  tone({ freq: 660, dur: 0.12, vol: 0.7 });
  tone({ freq: 990, dur: 0.16, vol: 0.7, delay: 0.09 });
}

/** Ошибка: низкий «бип-бип» */
export function playError(): void {
  tone({ freq: 330, dur: 0.14, type: "square", vol: 0.45 });
  tone({ freq: 233, dur: 0.22, type: "square", vol: 0.45, delay: 0.12 });
}

/** Запуск клиента: восходящий свип */
export function playLaunch(): void {
  tone({ freq: 340, dur: 0.28, type: "triangle", vol: 0.6, slideTo: 900 });
}

/** Краш клиента: нисходящий рёв */
export function playCrash(): void {
  tone({ freq: 440, dur: 0.3, type: "sawtooth", vol: 0.5, slideTo: 110 });
}

/* ------------------------------------------------------------------ */
/* Превью звуков в настройках                                          */
/* ------------------------------------------------------------------ */

export type SoundName = "click" | "success" | "error" | "launch" | "crash";

const PLAYERS: Record<SoundName, () => void> = {
  click: playClick,
  success: playSuccess,
  error: playError,
  launch: playLaunch,
  crash: playCrash,
};

/** Проиграть звук принудительно (кнопка проверки в настройках). */
export function previewSound(name: SoundName): void {
  const was = enabled;
  enabled = true;
  PLAYERS[name]();
  enabled = was;
}
