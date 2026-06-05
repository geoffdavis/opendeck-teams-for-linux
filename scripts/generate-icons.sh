#!/usr/bin/env bash
# Regenerate the plugin button icons with imagemagick.
# Ported from the original nix drawIcon derivations (nix-oceaneering).
set -euo pipefail
cd "$(dirname "$0")/../plugin/icons"

draw() { # size background output
	local s=$1 bg=$2 out=$3
	magick -size "${s}x${s}" "xc:${bg}" \
		-fill white -stroke white -strokewidth $((s / 36)) \
		-draw "roundrectangle $((s * 38 / 100)),$((s * 22 / 100)) \
		                      $((s * 62 / 100)),$((s * 58 / 100)) \
		                      $((s * 12 / 100)),$((s * 12 / 100))" \
		-fill none \
		-draw "arc $((s * 30 / 100)),$((s * 40 / 100)) \
		           $((s * 70 / 100)),$((s * 70 / 100)) 0,180" \
		-draw "line $((s * 50 / 100)),$((s * 65 / 100)) \
		            $((s * 50 / 100)),$((s * 80 / 100))" \
		-draw "line $((s * 38 / 100)),$((s * 80 / 100)) \
		            $((s * 62 / 100)),$((s * 80 / 100))" \
		"$out"
}

draw 72 "#4a3fcf" icon.png
draw 144 "#4a3fcf" icon@2x.png
draw 72 "#5a5a5a" icon-off.png
draw 144 "#5a5a5a" icon-off@2x.png

magick icon.png -fill "#ff4040" -stroke "#ff4040" -strokewidth 6 \
	-draw "line 14,58 58,14" icon-muted.png
magick icon@2x.png -fill "#ff4040" -stroke "#ff4040" -strokewidth 10 \
	-draw "line 28,116 116,28" icon-muted@2x.png

echo "icons regenerated"
