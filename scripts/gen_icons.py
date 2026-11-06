#!/usr/bin/env python3
"""Генератор иконок приложения (PIL): PNG для Tauri + многослойный .ico."""
from PIL import Image, ImageDraw
import os

OUT = os.path.join(os.path.dirname(__file__), "..", "src-tauri", "icons")

ACCENT_TOP = (139, 92, 246)   # #8b5cf6
ACCENT_BOT = (91, 47, 216)    # #5b2fd8


def diag_gradient(size, c1, c2):
    """Диагональный градиент (TL -> BR)."""
    w, h = size
    img = Image.new("RGB", (w, h))
    px = img.load()
    for y in range(h):
        for x in range(w):
            t = (x + y) / (w + h - 2)
            px[x, y] = tuple(
                round(c1[i] + (c2[i] - c1[i]) * t) for i in range(3)
            )
    return img


def make_icon(size: int) -> Image.Image:
    s = size
    # Базовый градиент
    img = diag_gradient((s, s), ACCENT_TOP, ACCENT_BOT).convert("RGBA")

    # Скруглённая маска на весь холст
    mask = Image.new("L", (s, s), 0)
    d = ImageDraw.Draw(mask)
    d.rounded_rectangle([0, 0, s - 1, s - 1], radius=round(s * 0.225), fill=255)
    img.putalpha(mask)

    draw = ImageDraw.Draw(img)

    # Мягкий блик сверху
    hl = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    dh = ImageDraw.Draw(hl)
    dh.ellipse(
        [round(-s * 0.25), round(-s * 0.45), round(s * 1.1), round(s * 0.55)],
        fill=(255, 255, 255, 42),
    )
    hl.putalpha(Image.composite(hl.split()[3], Image.new("L", (s, s), 0), mask))
    img = Image.alpha_composite(img, hl)

    draw = ImageDraw.Draw(img)

    # Два перекрывающихся окна (мотив мульти-инстанса)
    lw = max(2, round(s * 0.035))  # толщина линии
    r = round(s * 0.06)

    # Заднее окно (верх-лево)
    x1, y1, x2, y2 = round(s * 0.23), round(s * 0.28), round(s * 0.62), round(s * 0.60)
    draw.rounded_rectangle(
        [x1, y1, x2, y2], radius=r, outline=(255, 255, 255, 235), width=lw
    )
    # Полоска «заголовка»
    ty = y1 + round((y2 - y1) * 0.28)
    draw.line([(x1 + lw, ty), (x2 - lw, ty)], fill=(255, 255, 255, 200), width=lw // 2 + 1)

    # Переднее окно (низ-право)
    x1, y1, x2, y2 = round(s * 0.40), round(s * 0.44), round(s * 0.79), round(s * 0.76)
    draw.rounded_rectangle(
        [x1, y1, x2, y2],
        radius=r,
        fill=(10, 11, 18, 90),
        outline=(255, 255, 255, 150),
        width=lw,
    )
    ty = y1 + round((y2 - y1) * 0.28)
    draw.line([(x1 + lw, ty), (x2 - lw, ty)], fill=(255, 255, 255, 120), width=lw // 2 + 1)

    return img


def main():
    os.makedirs(OUT, exist_ok=True)
    big = make_icon(512)
    big.save(os.path.join(OUT, "icon.png"))
    big.resize((256, 256), Image.LANCZOS).save(os.path.join(OUT, "128x128@2x.png"))
    big.resize((128, 128), Image.LANCZOS).save(os.path.join(OUT, "128x128.png"))
    big.resize((32, 32), Image.LANCZOS).save(os.path.join(OUT, "32x32.png"))
    big.resize((16, 16), Image.LANCZOS).save(os.path.join(OUT, "16x16.png"))

    # Многослойный .ico
    ico_sizes = [256, 128, 64, 48, 32, 24, 16]
    frames = [make_icon(sz) if sz >= 64 else big.resize((sz, sz), Image.LANCZOS) for sz in ico_sizes]
    frames[0].save(os.path.join(OUT, "icon.ico"), format="ICO", sizes=[(sz, sz) for sz in ico_sizes])

    print("OK:", sorted(os.listdir(OUT)))


if __name__ == "__main__":
    main()
