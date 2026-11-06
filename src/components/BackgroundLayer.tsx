import { useSettings } from "../lib/store";

/**
 * Фоновый слой: кастомное изображение с прозрачностью и блюром,
 * либо амбиентная подсветка от акцентного цвета.
 */
export function BackgroundLayer() {
  const { bgImage, bgOpacity, bgBlur } = useSettings((s) => s.settings);

  return (
    <div className="fixed inset-0" aria-hidden>
      {bgImage ? (
        <>
          <img
            src={bgImage}
            alt=""
            className="absolute inset-0 h-full w-full object-cover"
            style={{
              filter: `blur(${bgBlur}px)`,
              opacity: bgOpacity / 100,
              transform: bgBlur > 0 ? "scale(1.08)" : undefined,
            }}
          />
          <div className="absolute inset-0 bg-[linear-gradient(180deg,rgb(4_5_9/0.30),rgb(4_5_9/0.55))]" />
        </>
      ) : (
        <div
          className="absolute inset-0"
          style={{
            background:
              "radial-gradient(55% 45% at 18% 8%, rgb(var(--accent-rgb) / 0.13), transparent 70%)," +
              "radial-gradient(45% 40% at 88% 92%, rgb(var(--accent-rgb) / 0.09), transparent 70%)," +
              "rgb(var(--c-bg))",
          }}
        />
      )}
      {/* Мягкая виньетка поверх */}
      <div className="absolute inset-0 bg-[radial-gradient(120%_120%_at_50%_50%,transparent_60%,rgb(0_0_0/0.35)_100%)]" />
    </div>
  );
}
