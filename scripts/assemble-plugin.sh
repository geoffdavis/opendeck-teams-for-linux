#!/usr/bin/env bash
# Assemble dist/com.geoffdavis.teamsforlinux.sdPlugin from plugin/ assets and
# built binaries. Prefers musl release binaries (CI); falls back to the native
# release binary for local smoke testing.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
dist="$root/dist/com.geoffdavis.teamsforlinux.sdPlugin"

rm -rf "$dist"
mkdir -p "$dist/bin"
cp -r "$root/plugin/." "$dist/"

copied=0
for arch in x86_64 aarch64; do
	bin="$root/target/${arch}-unknown-linux-musl/release/opendeck-teams-for-linux"
	if [[ -f "$bin" ]]; then
		cp "$bin" "$dist/bin/plugin-${arch}"
		copied=1
	fi
done

if [[ "$copied" -eq 0 ]]; then
	bin="$root/target/release/opendeck-teams-for-linux"
	if [[ ! -f "$bin" ]]; then
		echo "no built binaries found; run cargo build --release first" >&2
		exit 1
	fi
	cp "$bin" "$dist/bin/plugin-$(uname -m)"
fi

echo "assembled $dist"
