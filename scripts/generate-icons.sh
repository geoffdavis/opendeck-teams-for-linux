#!/usr/bin/env bash
# Regenerate the plugin's icons with Python + Pillow.
#
# Three families of icons are produced:
#   * plugin*.png      - the plugin's identity icon, based on the
#                        teams-for-linux application icon (vendored under
#                        assets/). Shown in OpenDeck's plugin list.
#   * icon*.png        - the mute action's button states (a microphone with a
#                        Teams "T"): normal, muted (red slash) and off (greyed).
#   * cam*.png         - the camera action's button states (a camcorder with a
#                        Teams "T"): on, off (red slash) and disabled (greyed).
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


def teams_t(d, s, cx, cy, color):
    """A small Teams "T" centred at (cx, cy)."""
    bw, th = s * 0.135, s * 0.042
    d.rounded_rectangle([cx - bw / 2, cy - s * 0.085, cx + bw / 2, cy - s * 0.085 + th],
                        radius=th * 0.3, fill=color)                 # bar
    d.rounded_rectangle([cx - th / 2, cy - s * 0.085, cx + th / 2, cy + s * 0.085],
                        radius=th * 0.3, fill=color)                 # stem


def draw_mic(d, s, color, tcolor):
    """Microphone with a Teams "T" cut into the capsule."""
    sw = max(1, round(s / 36))
    d.rounded_rectangle([s * 0.38, s * 0.22, s * 0.62, s * 0.58],
                        radius=s * 0.12, fill=color)                 # capsule
    d.arc([s * 0.30, s * 0.40, s * 0.70, s * 0.70], 0, 180, fill=color, width=sw)  # cradle
    d.line([s * 0.50, s * 0.64, s * 0.50, s * 0.80], fill=color, width=sw)         # stem
    d.line([s * 0.38, s * 0.80, s * 0.62, s * 0.80], fill=color, width=sw)         # base
    teams_t(d, s, s * 0.50, s * 0.355, tcolor)


def draw_cam(d, s, color, tcolor):
    """Camcorder (body + lens horn) with a Teams "T" cut into the body."""
    d.rounded_rectangle([s * 0.26, s * 0.40, s * 0.60, s * 0.64],
                        radius=s * 0.05, fill=color)                 # body
    d.polygon([(s * 0.60, s * 0.46), (s * 0.74, s * 0.40),
               (s * 0.74, s * 0.64), (s * 0.60, s * 0.58)], fill=color)  # lens horn
    teams_t(d, s, s * 0.40, s * 0.52, tcolor)


def action_icon(size, bg, fg, tcolor, glyph, slash=False):
    s = size * SS
    img = Image.new("RGBA", (s, s), bg)   # full-bleed background
    d = ImageDraw.Draw(img)
    glyph(d, s, fg, tcolor)
    if slash:
        d.line([s * 0.18, s * 0.82, s * 0.82, s * 0.18], fill=RED, width=round(s / 12))
    return img.resize((size, size), Image.LANCZOS)


def save(img, name):
    img.save(f"{ICONS}/{name}", optimize=True)
    print("wrote", name)


# Per-action button states (also embedded by the Rust binary via include_bytes!).
# (size, suffix) pairs for the 1x / 2x variants.
for size, sfx in ((72, ""), (144, "@2x")):
    # Mute: normal / muted (red slash) / off (greyed).
    save(action_icon(size, TFL, WHITE, TFL, draw_mic), f"icon{sfx}.png")
    save(action_icon(size, TFL, WHITE, TFL, draw_mic, slash=True), f"icon-muted{sfx}.png")
    save(action_icon(size, OFF, GREY, OFF, draw_mic), f"icon-off{sfx}.png")
    # Camera: on / off (red slash) / disabled (greyed).
    save(action_icon(size, TFL, WHITE, TFL, draw_cam), f"cam{sfx}.png")
    save(action_icon(size, TFL, WHITE, TFL, draw_cam, slash=True), f"cam-off{sfx}.png")
    save(action_icon(size, OFF, GREY, OFF, draw_cam), f"cam-disabled{sfx}.png")

# Plugin identity icon, based on the teams-for-linux application icon.
tfl = Image.open(SOURCE).convert("RGBA")
save(tfl.resize((72, 72), Image.LANCZOS), "plugin.png")
save(tfl.resize((144, 144), Image.LANCZOS), "plugin@2x.png")

print("icons regenerated")
PY
