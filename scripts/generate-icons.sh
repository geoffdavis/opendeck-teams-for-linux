#!/usr/bin/env bash
# Regenerate the plugin's icons with Python + Pillow.
#
# Two families of icons are produced:
#   * plugin*.png      - the plugin's identity icon, based on the
#                        teams-for-linux application icon (vendored under
#                        assets/). Shown in OpenDeck's plugin list.
#   * icon*.png        - the mute action's button states (a microphone with a
#                        Teams "T"): normal, muted (red slash) and off (greyed).
set -euo pipefail
cd "$(dirname "$0")/.."

python3 - <<'PY'
from PIL import Image, ImageDraw

ICONS = "plugin/icons"
SOURCE = "assets/teams-for-linux-icon.png"

TFL   = (75, 83, 188, 255)   # teams-for-linux brand purple (#4B53BC)
OFF   = (90, 90, 90, 255)    # greyed - not in a call
GREY  = (210, 210, 210, 255)
WHITE = (255, 255, 255, 255)
RED   = (255, 64, 64, 255)
SS    = 4                    # supersample factor for crisp anti-aliasing


def draw_mic(d, s, color, tcolor):
    """Microphone with a Teams "T" cut into the capsule."""
    sw = max(1, round(s / 36))
    # capsule
    d.rounded_rectangle([s * 0.38, s * 0.22, s * 0.62, s * 0.58],
                        radius=s * 0.12, fill=color)
    # cradle arc (lower half)
    d.arc([s * 0.30, s * 0.40, s * 0.70, s * 0.70], 0, 180, fill=color, width=sw)
    # stem + base
    d.line([s * 0.50, s * 0.64, s * 0.50, s * 0.80], fill=color, width=sw)
    d.line([s * 0.38, s * 0.80, s * 0.62, s * 0.80], fill=color, width=sw)
    # "T" cut into the upper capsule
    cx, cy = s * 0.50, s * 0.355
    bw, th = s * 0.135, s * 0.042
    d.rounded_rectangle([cx - bw / 2, cy - s * 0.085, cx + bw / 2, cy - s * 0.085 + th],
                        radius=th * 0.3, fill=tcolor)               # bar
    d.rounded_rectangle([cx - th / 2, cy - s * 0.085, cx + th / 2, cy + s * 0.085],
                        radius=th * 0.3, fill=tcolor)               # stem


def action_icon(size, bg, mic, tcolor, muted=False):
    s = size * SS
    img = Image.new("RGBA", (s, s), bg)   # full-bleed background
    d = ImageDraw.Draw(img)
    draw_mic(d, s, mic, tcolor)
    if muted:
        d.line([s * 0.18, s * 0.82, s * 0.82, s * 0.18], fill=RED, width=round(s / 12))
    return img.resize((size, size), Image.LANCZOS)


def save(img, name):
    img.save(f"{ICONS}/{name}", optimize=True)
    print("wrote", name)


# Mute action states (also embedded by the Rust binary via include_bytes!).
save(action_icon(72,  TFL, WHITE, TFL),               "icon.png")
save(action_icon(144, TFL, WHITE, TFL),               "icon@2x.png")
save(action_icon(72,  TFL, WHITE, TFL, muted=True),   "icon-muted.png")
save(action_icon(144, TFL, WHITE, TFL, muted=True),   "icon-muted@2x.png")
save(action_icon(72,  OFF, GREY,  OFF),               "icon-off.png")
save(action_icon(144, OFF, GREY,  OFF),               "icon-off@2x.png")

# Plugin identity icon, based on the teams-for-linux application icon.
tfl = Image.open(SOURCE).convert("RGBA")
save(tfl.resize((72, 72),   Image.LANCZOS), "plugin.png")
save(tfl.resize((144, 144), Image.LANCZOS), "plugin@2x.png")

print("icons regenerated")
PY
