"""Gera o ícone do app (1024x1024, grade de ícones do macOS).

Uso: python3 scripts/generate-icon.py && bun run tauri icon src-tauri/icons/app-icon.png
"""
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter

SS = 4  # supersampling pra antialias
S = 1024 * SS
OUT = Path(__file__).resolve().parent.parent / "src-tauri/icons/app-icon.png"


def p(v: float) -> int:
    return round(v * SS)


def vgradient(top: tuple, bottom: tuple, size=(S, S)) -> Image.Image:
    mask = Image.linear_gradient("L").resize(size)
    return Image.composite(Image.new("RGBA", size, bottom), Image.new("RGBA", size, top), mask)


def hgradient(stops: list[tuple], size=(S, S)) -> Image.Image:
    """Degradê horizontal com vários pontos de cor."""
    w, h = size
    strip = Image.new("RGBA", (len(stops) * 64, 1))
    px = strip.load()
    n = strip.width
    for x in range(n):
        t = x / (n - 1) * (len(stops) - 1)
        i = min(int(t), len(stops) - 2)
        f = t - i
        a, b = stops[i], stops[i + 1]
        px[x, 0] = tuple(round(a[k] + (b[k] - a[k]) * f) for k in range(4))
    return strip.resize((w, h), Image.BICUBIC)


# corpo do ícone: 824x824 em (100,100), raio ~185 (template do macOS)
X0, Y0, X1, Y1, R = 100, 100, 924, 924, 185
body = Image.new("L", (S, S), 0)
ImageDraw.Draw(body).rounded_rectangle((p(X0), p(Y0), p(X1), p(Y1)), radius=p(R), fill=255)

canvas = Image.new("RGBA", (S, S), (0, 0, 0, 0))

# sombra
shadow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
shadow.paste((0, 0, 0, 120), (0, p(14)), body)
canvas = Image.alpha_composite(canvas, shadow.filter(ImageFilter.GaussianBlur(p(22))))

# fundo: índigo profundo com brilho violeta embaixo à direita
bg = vgradient((40, 34, 82, 255), (9, 9, 22, 255))
glow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
ImageDraw.Draw(glow).ellipse((p(430), p(520), p(1100), p(1150)), fill=(124, 92, 255, 150))
bg = Image.alpha_composite(bg, glow.filter(ImageFilter.GaussianBlur(p(120))))
glow2 = Image.new("RGBA", (S, S), (0, 0, 0, 0))
ImageDraw.Draw(glow2).ellipse((p(-150), p(600), p(420), p(1100)), fill=(34, 211, 238, 70))
bg = Image.alpha_composite(bg, glow2.filter(ImageFilter.GaussianBlur(p(140))))

# Island: preta, colada no topo, com Ears côncavas
# largura cabe no trecho reto do topo (285..739) pra as Ears não invadirem a curva
CX, IW, IH, IR, E = 512, 400, 430, 150, 28
island = Image.new("L", (S, S), 0)
d = ImageDraw.Draw(island)
# começa bem acima do topo: os cantos arredondados de cima ficam fora, laterais retas até a borda
d.rounded_rectangle((p(CX - IW / 2), p(Y0 - 2 * IR), p(CX + IW / 2), p(Y0 + IH)), radius=p(IR), fill=255)
for side in (-1, 1):
    edge = CX + side * IW / 2
    ear = Image.new("L", (S, S), 0)
    ed = ImageDraw.Draw(ear)
    ex0, ex1 = sorted((edge, edge + side * E))
    ed.rectangle((p(ex0), p(Y0), p(ex1), p(Y0 + E)), fill=255)
    ccx = edge + side * E
    ed.ellipse((p(ccx - E), p(Y0), p(ccx + E), p(Y0 + 2 * E)), fill=0)
    island = ImageChops.lighter(island, ear)
bg.paste((0, 0, 0, 255), (0, 0), island)

# câmera do notch
cam = (CX, Y0 + 46, 14)
ImageDraw.Draw(bg).ellipse(
    (p(cam[0] - cam[2]), p(cam[1] - cam[2]), p(cam[0] + cam[2]), p(cam[1] + cam[2])), fill=(28, 28, 40, 255)
)

# equalizer em degradê rosa -> violeta -> ciano
bars = Image.new("L", (S, S), 0)
bd = ImageDraw.Draw(bars)
heights = [112, 196, 256, 176, 124]
BW, GAP, BCY = 40, 26, Y0 + 250
total = len(heights) * BW + (len(heights) - 1) * GAP
x = BX = CX - total / 2
for h in heights:
    bd.rounded_rectangle((p(x), p(BCY - h / 2), p(x + BW), p(BCY + h / 2)), radius=p(BW / 2), fill=255)
    x += BW + GAP
# degradê só na largura das barras, pra cada uma ter um tom
eq = Image.new("RGBA", (S, S), (0, 0, 0, 0))
eq.paste(hgradient([(244, 114, 182, 255), (167, 139, 250, 255), (34, 211, 238, 255)], (p(total), S)), (p(BX), 0))
bg.paste(eq, (0, 0), bars)

# brilho sutil no topo + aro interno
gloss = vgradient((255, 255, 255, 26), (255, 255, 255, 0))
bg = Image.alpha_composite(bg, Image.composite(gloss, Image.new("RGBA", (S, S), (0, 0, 0, 0)), ImageChops.subtract(body, island)))
rim = Image.new("L", (S, S), 0)
ImageDraw.Draw(rim).rounded_rectangle((p(X0), p(Y0), p(X1), p(Y1)), radius=p(R), outline=255, width=p(3))
bg.paste((255, 255, 255, 30), (0, 0), ImageChops.subtract(rim, island))

icon = Image.new("RGBA", (S, S), (0, 0, 0, 0))
icon.paste(bg, (0, 0), body)
canvas = Image.alpha_composite(canvas, icon)
canvas.resize((1024, 1024), Image.LANCZOS).save(OUT, optimize=True)
print(OUT)
