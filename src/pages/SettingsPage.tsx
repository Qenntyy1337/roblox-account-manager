import {
  Bell,
  Database,
  FolderOpen,
  Image as ImageIcon,
  Info,
  Languages,
  LogIn,
  Palette,
  RotateCcw,
  Rocket,
  ShieldAlert,
  Trash2,
  TriangleAlert,
  Volume2,
} from "lucide-react";
import { useState } from "react";
import { cmd, isTauri } from "../lib/bridge";
import { LANGUAGES } from "../lib/i18n";
import { previewSound, type SoundName } from "../lib/sound";
import { toast, useSettings, useT } from "../lib/store";
import { PALETTES } from "../lib/theme";
import {
  Button,
  ColorPicker,
  Field,
  IconButton,
  SectionCard,
  Slider,
  TextField,
  Toggle,
  cn,
} from "../components/ui";

/** Кнопки проверки звука в настройках */
const SOUND_PREVIEWS: { id: SoundName; label: string }[] = [
  { id: "click", label: "settings.sound.sClick" },
  { id: "success", label: "settings.sound.sSuccess" },
  { id: "error", label: "settings.sound.sError" },
  { id: "launch", label: "settings.sound.sLaunch" },
  { id: "crash", label: "settings.sound.sCrash" },
];

export function SettingsPage() {
  const t = useT();
  const { settings, patch } = useSettings();
  const [confirmReset, setConfirmReset] = useState(false);

  const pickImage = async () => {
    try {
      const picked = await cmd<string | null>("pick_file", { kind: "image" });
      if (picked) {
        let src = picked;
        if (isTauri && !picked.startsWith("blob:") && !picked.startsWith("data:")) {
          const { convertFileSrc } = await import("@tauri-apps/api/core");
          src = convertFileSrc(picked);
        }
        await patch({ bgImage: src });
        toast("success", t("toast.settingsSaved"));
      }
    } catch (e) {
      toast("error", String(e));
    }
  };

  return (
    <div className="flex h-full flex-col">
      <div className="px-6 pb-2 pt-2">
        <h1 className="text-[22px] font-bold tracking-tight text-ink">{t("settings.title")}</h1>
        <p className="text-[13px] text-faint">{t("settings.subtitle")}</p>
      </div>

      <div className="ram-scroll flex-1 overflow-y-auto px-6 pb-6 pt-2">
        <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
          {/* ---------- Внешний вид ---------- */}
          <SectionCard icon={<Palette size={16} />} title={t("settings.section.appearance")}>
            <div className="flex flex-col gap-5">
              <div className="flex flex-col gap-3">
                <div className="text-[13px] font-medium text-mut">{t("settings.accent")}</div>
                <div className="text-[12px] text-faint">{t("settings.presets")}</div>
                <div className="flex flex-wrap gap-2.5">
                  {PALETTES.map((p) => (
                    <button
                      key={p.id}
                      title={p.name}
                      onClick={() => void patch({ accent: p.hex })}
                      className={cn(
                        "h-8 w-8 rounded-full transition-all duration-150 hover:scale-110",
                        settings.accent.toLowerCase() === p.hex.toLowerCase()
                          ? "ring-2 ring-white/90 ring-offset-2 ring-offset-[rgb(14_16_24)]"
                          : "ring-1 ring-white/15",
                      )}
                      style={{ background: p.hex }}
                    />
                  ))}
                </div>

                <div className="mt-1 text-[12px] text-faint">{t("settings.custom")}</div>
                <div className="rounded-xl border border-line bg-surface/50 p-3.5">
                  <ColorPicker value={settings.accent} onChange={(hex) => void patch({ accent: hex })} />
                </div>
              </div>

              <div className="flex flex-col gap-3 border-t border-line/70 pt-4">
                <div className="text-[13px] font-medium text-mut">{t("settings.background")}</div>
                {settings.bgImage ? (
                  <div className="relative overflow-hidden rounded-xl border border-line">
                    <img src={settings.bgImage} alt="" className="h-28 w-full object-cover" />
                    <IconButton
                      label={t("settings.bg.remove")}
                      danger
                      className="absolute right-2 top-2 border border-line bg-black/60"
                      onClick={() => void patch({ bgImage: null })}
                    >
                      <Trash2 size={15} />
                    </IconButton>
                  </div>
                ) : (
                  <button
                    onClick={() => void pickImage()}
                    className="flex h-24 w-full flex-col items-center justify-center gap-1.5 rounded-xl border border-dashed border-line text-faint transition-all hover:border-accent/50 hover:text-mut"
                  >
                    <ImageIcon size={20} />
                    <span className="text-[13px]">{t("action.browse")}</span>
                  </button>
                )}
                <p className="text-xs leading-snug text-faint">{t("settings.bg.hint")}</p>

                <Field label={t("settings.bg.opacity")}>
                  <Slider
                    value={settings.bgOpacity}
                    min={5}
                    max={100}
                    unit="%"
                    disabled={!settings.bgImage}
                    onChange={(v) => void patch({ bgOpacity: v })}
                  />
                </Field>
                <Field label={t("settings.bg.blur")}>
                  <Slider
                    value={settings.bgBlur}
                    min={0}
                    max={40}
                    unit="px"
                    disabled={!settings.bgImage}
                    onChange={(v) => void patch({ bgBlur: v })}
                  />
                </Field>
              </div>

              <div className="flex items-start justify-between gap-4 rounded-xl border border-line bg-surface/50 p-3.5">
                <div className="flex items-start gap-2.5">
                  <Volume2 size={16} className="mt-0.5 shrink-0 text-mut" />
                  <div>
                    <div className="text-[13px] font-medium text-ink">{t("settings.sound")}</div>
                    <div className="mt-0.5 text-xs leading-snug text-faint">
                      {t("settings.sound.desc")}
                    </div>
                  </div>
                </div>
                <Toggle
                  checked={settings.soundEnabled}
                  onChange={(v) => void patch({ soundEnabled: v })}
                  label={t("settings.sound")}
                />
              </div>

              {/* Проверка звуковых эффектов */}
              <div className="flex flex-wrap items-center gap-2 pl-1">
                <span className="mr-1 text-[12px] text-faint">{t("settings.sound.preview")}</span>
                {SOUND_PREVIEWS.map((s) => (
                  <button
                    key={s.id}
                    onClick={() => previewSound(s.id)}
                    title={t(s.label)}
                    className={cn(
                      "rounded-full border border-line bg-surface/60 px-2.5 py-1 text-[11px] text-mut",
                      "transition-all duration-150 hover:border-accent/50 hover:text-ink active:scale-95",
                    )}
                  >
                    {t(s.label)}
                  </button>
                ))}
              </div>

              {/* Уведомления Windows */}
              <div className="flex items-start justify-between gap-4 rounded-xl border border-line bg-surface/50 p-3.5">
                <div className="flex items-start gap-2.5">
                  <Bell size={16} className="mt-0.5 shrink-0 text-mut" />
                  <div>
                    <div className="text-[13px] font-medium text-ink">{t("settings.notifications")}</div>
                    <div className="mt-0.5 text-xs leading-snug text-faint">
                      {t("settings.notifications.desc")}
                    </div>
                  </div>
                </div>
                <Toggle
                  checked={settings.winNotifications}
                  onChange={(v) => void patch({ winNotifications: v })}
                  label={t("settings.notifications")}
                />
              </div>
            </div>
          </SectionCard>

          {/* ---------- Язык + Запуск ---------- */}
          <div className="flex flex-col gap-4">
            <SectionCard icon={<Languages size={16} />} title={t("settings.section.language")}>
              <div className="grid grid-cols-3 gap-2">
                {LANGUAGES.map((l) => (
                  <button
                    key={l.id}
                    onClick={() => void patch({ language: l.id })}
                    className={cn(
                      "rounded-xl border px-3 py-2.5 text-sm font-medium transition-all duration-150",
                      settings.language === l.id
                        ? "border-accent/60 bg-accent/12 text-accent shadow-glowSm"
                        : "border-line text-mut hover:border-accent/40 hover:text-ink",
                    )}
                  >
                    {l.native}
                  </button>
                ))}
              </div>
            </SectionCard>

            <SectionCard icon={<Rocket size={16} />} title={t("settings.section.launcher")}>
              <div className="flex flex-col gap-4">
                <Field
                  id="rbx-path"
                  label={t("settings.launcher.path")}
                  hint={t("settings.launcher.path.hint")}
                >
                  <div className="flex items-center gap-2">
                    <TextField
                      id="rbx-path"
                      value={settings.robloxPath ?? ""}
                      placeholder="C:\Users\…\Roblox\Versions\…\RobloxPlayerBeta.exe"
                      className="font-mono text-[12px]"
                      onChange={(e) => void patch({ robloxPath: e.target.value || null })}
                    />
                    <Button
                      size="md"
                      onClick={async () => {
                        try {
                          const p = await cmd<string | null>("detect_roblox_path", {});
                          if (p) {
                            await patch({ robloxPath: p });
                            toast("success", t("toast.settingsSaved"));
                          }
                        } catch (e) {
                          toast("error", String(e));
                        }
                      }}
                    >
                      {t("action.detect")}
                    </Button>
                  </div>
                </Field>

                <Field label={t("settings.launcher.delay")} hint={t("settings.launcher.delay.hint")}>
                  <Slider
                    value={settings.launchDelayMs}
                    min={200}
                    max={4000}
                    step={100}
                    unit="ms"
                    onChange={(v) => void patch({ launchDelayMs: v })}
                  />
                </Field>

                <div className="flex items-start justify-between gap-4 rounded-xl border border-line bg-surface/50 p-3.5">
                  <div className="flex items-start gap-2.5">
                    <LogIn size={16} className="mt-0.5 shrink-0 text-mut" />
                    <div>
                      <div className="text-[13px] font-medium text-ink">
                        {t("settings.launcher.autologin")}
                      </div>
                      <div className="mt-0.5 text-xs leading-snug text-faint">
                        {t("settings.launcher.autologin.desc")}
                      </div>
                      <div className="mt-1 text-[11px] leading-snug text-warn/80">
                        {t("settings.launcher.autologin.hint")}
                      </div>
                    </div>
                  </div>
                  <Toggle
                    checked={settings.autoLogin}
                    onChange={(v) => void patch({ autoLogin: v })}
                    label={t("settings.launcher.autologin")}
                  />
                </div>

                <div className="flex items-start justify-between gap-4 rounded-xl border border-line bg-surface/50 p-3.5">
                  <div>
                    <div className="text-[13px] font-medium text-ink">
                      {t("settings.launcher.isolate")}
                    </div>
                    <div className="mt-0.5 text-xs leading-snug text-faint">
                      {t("settings.launcher.isolate.desc")}
                    </div>
                  </div>
                  <Toggle
                    checked={settings.isolateData}
                    onChange={(v) => void patch({ isolateData: v })}
                    label={t("settings.launcher.isolate")}
                  />
                </div>

                <div className="flex items-start justify-between gap-4 rounded-xl border border-line bg-surface/50 p-3.5">
                  <div>
                    <div className="text-[13px] font-medium text-ink">
                      {t("settings.launcher.closeSingleton")}
                    </div>
                    <div className="mt-0.5 text-xs leading-snug text-faint">
                      {t("settings.launcher.closeSingleton.desc")}
                    </div>
                  </div>
                  <Toggle
                    checked={settings.closeSingleton}
                    onChange={(v) => void patch({ closeSingleton: v })}
                    label={t("settings.launcher.closeSingleton")}
                  />
                </div>

                <div className="flex items-start gap-2.5 rounded-xl border border-warn/30 bg-warn/[0.08] p-3.5">
                  <TriangleAlert size={16} className="mt-0.5 shrink-0 text-warn" />
                  <p className="text-xs leading-relaxed text-warn/90">{t("settings.launcher.risk")}</p>
                </div>
              </div>
            </SectionCard>
          </div>
        </div>

        {/* ---------- Данные + О программе ---------- */}
        <div className="mt-4 grid grid-cols-1 gap-4 xl:grid-cols-2">
          <SectionCard icon={<Database size={16} />} title={t("settings.section.data")}>
            <div className="flex flex-col gap-3">
              <div className="flex items-start justify-between gap-4 rounded-xl border border-line bg-surface/50 p-3.5">
                <div className="flex items-start gap-2.5">
                  <FolderOpen size={16} className="mt-0.5 shrink-0 text-mut" />
                  <div>
                    <div className="text-[13px] font-medium text-ink">{t("settings.data.folder")}</div>
                    <div className="mt-0.5 text-xs leading-snug text-faint">
                      {t("settings.data.folder.desc")}
                    </div>
                  </div>
                </div>
                <Button
                  size="sm"
                  onClick={async () => {
                    try {
                      await cmd("open_path", { path: "__appdata__" });
                    } catch (e) {
                      toast("error", String(e));
                    }
                  }}
                >
                  {t("settings.data.open")}
                </Button>
              </div>

              <div className="flex items-start justify-between gap-4 rounded-xl border border-line bg-surface/50 p-3.5">
                <div className="flex items-start gap-2.5">
                  <RotateCcw size={16} className="mt-0.5 shrink-0 text-mut" />
                  <div>
                    <div className="text-[13px] font-medium text-ink">{t("settings.data.reset")}</div>
                    <div className="mt-0.5 text-xs leading-snug text-faint">
                      {t("settings.data.reset.desc")}
                    </div>
                  </div>
                </div>
                <Button size="sm" variant="danger" onClick={() => setConfirmReset(true)}>
                  {t("settings.data.reset")}
                </Button>
              </div>
            </div>
          </SectionCard>

          <SectionCard icon={<Info size={16} />} title={t("settings.section.about")}>
            <div className="flex flex-col gap-2.5 text-[13px] leading-relaxed text-mut">
              <div className="flex items-center gap-2">
                <span className="text-faint">{t("settings.about.version")}:</span>
                <span className="rounded-md border border-accent/40 bg-accent/10 px-2 py-0.5 font-mono text-xs text-accent">
                  1.0.1
                </span>
              </div>
              <p>{t("settings.about.stack")}</p>
              <p className="flex items-start gap-2 text-faint">
                <ShieldAlert size={14} className="mt-0.5 shrink-0" />
                {t("settings.about.disclaimer")}
              </p>
            </div>
          </SectionCard>
        </div>
      </div>

      {/* Подтверждение сброса настроек */}
      {confirmReset && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/55 backdrop-blur-sm animate-fadeIn"
          onMouseDown={(e) => {
            if (e.target === e.currentTarget) setConfirmReset(false);
          }}
        >
          <div className="w-[400px] rounded-2xl border border-line bg-[rgb(17_19_27/0.97)] p-5 shadow-modal animate-scaleIn">
            <h3 className="text-[15px] font-semibold text-ink">{t("settings.data.reset")}</h3>
            <p className="mt-2 text-sm leading-relaxed text-mut">{t("settings.data.reset.desc")}</p>
            <div className="mt-4 flex justify-end gap-2">
              <Button variant="ghost" onClick={() => setConfirmReset(false)}>
                {t("action.cancel")}
              </Button>
              <Button
                variant="danger"
                onClick={() => {
                  void patch({
                    accent: "#8b5cf6",
                    bgImage: null,
                    bgOpacity: 55,
                    bgBlur: 14,
                    launchDelayMs: 900,
                  }).then(() => toast("success", t("toast.reset")));
                  setConfirmReset(false);
                }}
              >
                {t("confirm.yes")}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
