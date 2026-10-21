"""Genera los iconos de Wispr Swag (aplicación y bandeja) en blanco y negro.

Uso: python3 tools/gen_icons.py   (requiere Pillow)
"""
from PIL import Image, ImageDraw
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "src-tauri", "icons")
os.makedirs(OUT, exist_ok=True)
SS = 8  # supermuestreo para bordes suaves

BARS = [0.34, 0.62, 1.0, 0.72, 0.42]


def app_icon(size):
    S = size * SS
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    r = int(S * 0.235)
    d.rounded_rectangle([0, 0, S - 1, S - 1], radius=r, fill=(12, 12, 13, 255))
    # Píldora interior
    pw, ph = S * 0.66, S * 0.30
    px, py = (S - pw) / 2, (S - ph) / 2
    d.rounded_rectangle([px, py, px + pw, py + ph], radius=ph / 2, fill=(245, 245, 245, 255))
    # Onda
    bw = S * 0.042
    gap = S * 0.036
    total = len(BARS) * bw + (len(BARS) - 1) * gap
    x = (S - total) / 2
    for h in BARS:
        bh = ph * 0.70 * h
        y = S / 2 - bh / 2
        d.rounded_rectangle([x, y, x + bw, y + bh], radius=bw / 2, fill=(12, 12, 13, 255))
        x += bw + gap
    return img.resize((size, size), Image.LANCZOS)


def tray_icon(size, color, recording=False, paused=False):
    S = size * SS
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    pw, ph = S * 0.92, S * 0.50
    px, py = (S - pw) / 2, (S - ph) / 2
    stroke = max(int(S * 0.075), 1)
    if recording:
        d.rounded_rectangle([px, py, px + pw, py + ph], radius=ph / 2, fill=color + (255,))
        bar_color = (0, 0, 0, 0)
    else:
        d.rounded_rectangle([px, py, px + pw, py + ph], radius=ph / 2, outline=color + (255,), width=stroke)
        bar_color = color + (255,)
    bars = [0.45, 0.8, 1.0, 0.8, 0.45] if not paused else [0.0, 1.0, 0.0, 1.0, 0.0]
    bw = S * 0.075
    gap = S * 0.06
    total = len(bars) * bw + (len(bars) - 1) * gap
    x = (S - total) / 2
    for h in bars:
        if h > 0:
            bh = ph * 0.58 * h
            y = S / 2 - bh / 2
            d.rounded_rectangle([x, y, x + bw, y + bh], radius=bw / 2, fill=bar_color)
        x += bw + gap
    if recording:
        # Recorta las barras (transparentes) sobre el relleno.
        mask = Image.new("L", (S, S), 0)
        md = ImageDraw.Draw(mask)
        x = (S - total) / 2
        for h in bars:
            bh = ph * 0.58 * h
            y = S / 2 - bh / 2
            md.rounded_rectangle([x, y, x + bw, y + bh], radius=bw / 2, fill=255)
            x += bw + gap
        alpha = img.split()[3]
        alpha = Image.composite(Image.new("L", (S, S), 0), alpha, mask)
        img.putalpha(alpha)
    return img.resize((size, size), Image.LANCZOS)


app_icon(32).save(os.path.join(OUT, "32x32.png"))
app_icon(128).save(os.path.join(OUT, "128x128.png"))
app_icon(256).save(os.path.join(OUT, "128x128@2x.png"))
app_icon(512).save(os.path.join(OUT, "icon.png"))
big = app_icon(256)
big.save(os.path.join(OUT, "icon.ico"), sizes=[(16, 16), (20, 20), (24, 24), (32, 32), (40, 40), (48, 48), (64, 64), (128, 128), (256, 256)])
for name, color in (("light", (255, 255, 255)), ("dark", (20, 20, 20))):
    tray_icon(32, color).save(os.path.join(OUT, f"tray-idle-{name}.png"))
    tray_icon(32, color, recording=True).save(os.path.join(OUT, f"tray-rec-{name}.png"))
    tray_icon(32, color, paused=True).save(os.path.join(OUT, f"tray-paused-{name}.png"))
# Icono para la interfaz web
os.makedirs(os.path.join(ROOT, "public"), exist_ok=True)
app_icon(64).save(os.path.join(ROOT, "public", "icon.png"))
print("iconos generados en", OUT)
