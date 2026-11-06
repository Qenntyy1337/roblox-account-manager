import type {
  ButtonHTMLAttributes,
  InputHTMLAttributes,
  ReactNode,
  TextareaHTMLAttributes,
} from "react";
import { useEffect, useId, useRef, useState } from "react";

export const cn = (...xs: (string | false | null | undefined)[]) =>
  xs.filter(Boolean).join(" ");

/* ----------------------------- Кнопки ----------------------------- */

interface BtnProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "ghost" | "subtle" | "danger";
  size?: "sm" | "md";
  icon?: ReactNode;
}

export function Button({
  variant = "subtle",
  size = "md",
  icon,
  className,
  children,
  ...rest
}: BtnProps) {
  const base =
    "inline-flex items-center justify-center gap-2 rounded-lg font-medium transition-all duration-150 select-none disabled:opacity-40 disabled:pointer-events-none active:scale-[0.98] whitespace-nowrap";
  const sizes = {
    sm: "h-8 px-3 text-[13px]",
    md: "h-[38px] px-4 text-sm",
  } as const;
  const variants = {
    primary:
      "bg-accent text-[color:var(--accent-ink)] hover:brightness-110 shadow-glowSm",
    ghost: "text-mut hover:text-ink hover:bg-white/[0.06]",
    subtle:
      "bg-surface/80 border border-line text-ink hover:border-accent/50 hover:bg-surface",
    danger: "bg-err/10 border border-err/30 text-err hover:bg-err/20",
  } as const;
  return (
    <button className={cn(base, sizes[size], variants[variant], className)} {...rest}>
      {icon}
      {children}
    </button>
  );
}

interface IconBtnProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  danger?: boolean;
  active?: boolean;
}

export function IconButton({ label, danger, active, className, children, ...rest }: IconBtnProps) {
  return (
    <button
      title={label}
      aria-label={label}
      className={cn(
        "inline-flex h-8 w-8 items-center justify-center rounded-lg border border-transparent text-mut transition-all duration-150",
        "hover:bg-white/[0.07] hover:text-ink active:scale-95",
        danger && "hover:bg-err/15 hover:text-err",
        active && "text-accent bg-accent/10",
        className,
      )}
      {...rest}
    >
      {children}
    </button>
  );
}

/* ----------------------------- Поля ------------------------------- */

interface FieldWrapProps {
  label?: string;
  hint?: string;
  children: ReactNode;
  id?: string;
}

export function Field({ label, hint, children, id }: FieldWrapProps) {
  return (
    <div className="flex flex-col gap-1.5">
      {label && (
        <label htmlFor={id} className="text-[13px] font-medium text-mut">
          {label}
        </label>
      )}
      {children}
      {hint && <p className="text-xs text-faint leading-snug">{hint}</p>}
    </div>
  );
}

const inputCls =
  "w-full rounded-lg border border-line bg-surface/70 px-3 text-sm text-ink placeholder:text-faint outline-none transition-all duration-150 focus:border-accent/60 focus:ring-2 focus:ring-[rgb(var(--accent-rgb)/0.18)]";

export function TextField(props: InputHTMLAttributes<HTMLInputElement>) {
  const { className, ...rest } = props;
  return <input className={cn(inputCls, "h-10", className)} {...rest} />;
}

export function TextArea(props: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  const { className, ...rest } = props;
  return <textarea className={cn(inputCls, "py-2.5 font-mono text-[13px]", className)} {...rest} />;
}

/* ---------------------------- Слайдер ------------------------------ */

interface SliderProps {
  value: number;
  min: number;
  max: number;
  step?: number;
  unit?: string;
  disabled?: boolean;
  onChange: (v: number) => void;
}

export function Slider({ value, min, max, step = 1, unit, disabled, onChange }: SliderProps) {
  const pct = ((value - min) / (max - min)) * 100;
  return (
    <div className="flex items-center gap-3">
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(Number(e.target.value))}
        className="ram-slider h-1.5 flex-1 cursor-pointer appearance-none rounded-full disabled:opacity-40"
        style={{
          background: `linear-gradient(90deg, rgb(var(--accent-rgb)) ${pct}%, rgb(255 255 255 / 0.10) ${pct}%)`,
        }}
      />
      <span className="w-14 shrink-0 text-right font-mono text-xs text-mut">
        {value}
        {unit}
      </span>
    </div>
  );
}

/* ----------------------------- Тумблер ----------------------------- */

interface ToggleProps {
  checked: boolean;
  disabled?: boolean;
  onChange: (v: boolean) => void;
  label?: string;
}

export function Toggle({ checked, disabled, onChange, label }: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        "relative h-6 w-11 shrink-0 rounded-full transition-all duration-200 outline-none",
        checked
          ? "bg-accent shadow-glowSm"
          : "bg-white/[0.12] hover:bg-white/[0.18]",
        disabled && "opacity-40 pointer-events-none",
      )}
    >
      <span
        className={cn(
          "absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition-all duration-200",
          checked ? "left-[22px]" : "left-0.5",
        )}
      />
    </button>
  );
}

/* ----------------------------- Селект ------------------------------ */

export interface SelectItem<T extends string> {
  value: T;
  label: string;
}

interface SelectProps<T extends string> {
  items: SelectItem<T>[];
  value: T | null;
  onChange: (v: T) => void;
  className?: string;
}

export function Select<T extends string>({ items, value, onChange, className }: SelectProps<T>) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const current = items.find((i) => i.value === value);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  return (
    <div ref={ref} className={cn("relative", className)}>
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        className={cn(
          inputCls,
          "h-9 flex items-center justify-between gap-2 pr-2 text-left",
          open && "border-accent/60",
        )}
      >
        <span className={cn("truncate", !current && "text-faint")}>
          {current?.label ?? "—"}
        </span>
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.2"
          strokeLinecap="round"
          strokeLinejoin="round"
          className={cn("shrink-0 text-faint transition-transform duration-150", open && "rotate-180")}
        >
          <path d="m6 9 6 6 6-6" />
        </svg>
      </button>
      {open && (
        <div className="absolute right-0 top-[calc(100%+6px)] z-40 min-w-full overflow-hidden rounded-xl border border-line bg-[rgb(20_22_30/0.97)] p-1 shadow-modal backdrop-blur-xl animate-scaleIn">
          {items.map((it) => (
            <button
              key={it.value}
              type="button"
              onClick={() => {
                onChange(it.value);
                setOpen(false);
              }}
              className={cn(
                "flex w-full items-center gap-2 rounded-lg px-3 py-1.5 text-left text-[13px] transition-colors",
                it.value === value
                  ? "bg-accent/15 text-accent"
                  : "text-mut hover:bg-white/[0.06] hover:text-ink",
              )}
            >
              {it.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

/* ---------------------------- EmptyState --------------------------- */

export function EmptyState({
  icon,
  title,
  hint,
  action,
}: {
  icon: ReactNode;
  title: string;
  hint?: string;
  action?: ReactNode;
}) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-3 py-16 text-center animate-fadeUp">
      <div className="flex h-14 w-14 items-center justify-center rounded-2xl border border-line bg-surface/60 text-faint">
        {icon}
      </div>
      <div className="text-[15px] font-medium text-mut">{title}</div>
      {hint && <div className="max-w-sm text-sm text-faint">{hint}</div>}
      {action && <div className="mt-2">{action}</div>}
    </div>
  );
}

/* --------------------------- SectionCard --------------------------- */

export function SectionCard({
  icon,
  title,
  children,
  className,
}: {
  icon: ReactNode;
  title: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section
      className={cn(
        "rounded-2xl border border-line bg-panel/60 p-5 shadow-card backdrop-blur-xl",
        className,
      )}
    >
      <div className="mb-4 flex items-center gap-2.5">
        <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-accent/12 text-accent">
          {icon}
        </div>
        <h2 className="text-[15px] font-semibold text-ink">{title}</h2>
      </div>
      {children}
    </section>
  );
}

/* ------------------------------ Модалка ---------------------------- */

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  footer?: ReactNode;
  width?: number;
}

export function Modal({ open, onClose, title, children, footer, width = 460 }: ModalProps) {
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/55 backdrop-blur-sm animate-fadeIn"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className="max-h-[85vh] overflow-hidden rounded-2xl border border-line bg-[rgb(17_19_27/0.96)] shadow-modal animate-scaleIn"
        style={{ width }}
      >
        <div className="flex items-center justify-between border-b border-line px-5 py-3.5">
          <h3 className="text-[15px] font-semibold text-ink">{title}</h3>
          <IconButton label="close" onClick={onClose}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <path d="M18 6 6 18M6 6l12 12" />
            </svg>
          </IconButton>
        </div>
        <div className="max-h-[62vh] overflow-y-auto px-5 py-4 ram-scroll">{children}</div>
        {footer && (
          <div className="flex items-center justify-end gap-2 border-t border-line px-5 py-3.5">
            {footer}
          </div>
        )}
      </div>
    </div>
  );
}

/* --------------------------- ColorPicker --------------------------- */

interface ColorPickerProps {
  value: string;
  onChange: (hex: string) => void;
}

export function ColorPicker({ value, onChange }: ColorPickerProps) {
  const id = useId();
  const [hexDraft, setHexDraft] = useState(value);
  useEffect(() => setHexDraft(value), [value]);

  const clamp = (v: number) => Math.max(0, Math.min(255, v));
  const setRgb = (i: 0 | 1 | 2, v: number) => {
    const m = /^#?([0-9a-f]{6})$/i.exec(value);
    const n = m ? parseInt(m[1], 16) : 0x8b5cf6;
    const c = [(n >> 16) & 255, (n >> 8) & 255, n & 255];
    c[i] = clamp(v);
    const hex = `#${c.map((x) => x.toString(16).padStart(2, "0")).join("")}`;
    onChange(hex);
  };
  const rgb = (() => {
    const m = /^#?([0-9a-f]{6})$/i.exec(value);
    const n = m ? parseInt(m[1], 16) : 0x8b5cf6;
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255] as const;
  })();

  const commitHex = () => {
    const m = /^#?([0-9a-f]{6})$/i.exec(hexDraft.trim());
    if (m) onChange(`#${m[1].toLowerCase()}`);
    else setHexDraft(value);
  };

  const row = (label: string, i: 0 | 1 | 2, color: string) => (
    <div className="flex items-center gap-2.5">
      <span className="w-4 text-center font-mono text-xs text-faint">{label}</span>
      <input
        type="range"
        min={0}
        max={255}
        value={rgb[i]}
        onChange={(e) => setRgb(i, Number(e.target.value))}
        className="ram-slider h-1.5 flex-1 cursor-pointer appearance-none rounded-full"
        style={{
          background: `linear-gradient(90deg, rgb(0 0 0) 0%, ${color} 100%)`,
        }}
      />
      <span className="w-9 text-right font-mono text-xs text-mut">{rgb[i]}</span>
    </div>
  );

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center gap-3">
        <label
          htmlFor={id}
          className="relative h-10 w-14 shrink-0 cursor-pointer overflow-hidden rounded-lg border border-line transition-transform hover:scale-[1.03]"
          style={{ background: value }}
        >
          <input
            id={id}
            type="color"
            value={value}
            onChange={(e) => onChange(e.target.value)}
            className="absolute inset-0 h-full w-full cursor-pointer opacity-0"
          />
        </label>
        <TextField
          value={hexDraft}
          spellCheck={false}
          onChange={(e) => setHexDraft(e.target.value)}
          onBlur={commitHex}
          onKeyDown={(e) => {
            if (e.key === "Enter") commitHex();
          }}
          className="font-mono uppercase"
        />
      </div>
      {row("R", 0, `rgb(${rgb[0]} 0 0)`)}
      {row("G", 1, `rgb(0 ${rgb[1]} 0)`)}
      {row("B", 2, `rgb(0 0 ${rgb[2]})`)}
    </div>
  );
}
