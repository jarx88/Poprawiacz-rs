#!/usr/bin/env bash
# Builds the Windows EXE on this Linux machine (cargo-xwin) and publishes it for
# the app's self-updater: <dir>/PoprawiaczTekstu-<build>.exe + <dir>/latest.json.
# HyperRust's update server (~/HyperRust/scripts/hyperrust-updates.service)
# serves ~/hyperrust-updates over Tailscale, so the default <dir> is its
# poprawiacz/ subdirectory; running copies pick the build up within ~2 min.
#
#   scripts/publish-dev.sh            # fast build (profile dev-release)
#   scripts/publish-dev.sh --release  # same optimisation as CI (fat LTO)
#
# Env: POPRAWIACZ_UPDATE_DIR (default ~/hyperrust-updates/poprawiacz), KEEP (default 5 builds).
set -euo pipefail

repo=$(cd "$(dirname "$0")/.." && pwd)
out=${POPRAWIACZ_UPDATE_DIR:-$HOME/hyperrust-updates/poprawiacz}
keep=${KEEP:-5}
profile=dev-release
[[ ${1:-} == --release ]] && profile=release
target=x86_64-pc-windows-msvc

# The cargo wrapper schedules builds through the user's systemd instance.
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
export DBUS_SESSION_BUS_ADDRESS=${DBUS_SESSION_BUS_ADDRESS:-unix:path=$XDG_RUNTIME_DIR/bus}

cd "$repo"
built_at=$(date +%s)
sha=$(git rev-parse --short HEAD)
git diff --quiet HEAD || sha="$sha-dirty"
export POPRAWIACZ_BUILT_AT=$built_at POPRAWIACZ_BUILD="$sha-$built_at"
notes=$(git log -1 --format=%s)

pnpm install --frozen-lockfile >/dev/null
pnpm build
# Plain cargo (not the Tauri CLI) so the custom profile works; custom-protocol
# embeds dist/ instead of pointing the webview at the dev server.
(cd src-tauri && cargo xwin build --profile "$profile" --target "$target" --features tauri/custom-protocol)

exe="src-tauri/target/$target/$profile/poprawiacz.exe"
[[ -f $exe ]] || { echo "brak $exe" >&2; exit 1; }

mkdir -p "$out"
name="PoprawiaczTekstu-$POPRAWIACZ_BUILD.exe"
cp "$exe" "$out/$name.part" && mv "$out/$name.part" "$out/$name"
sum=$(sha256sum "$out/$name" | cut -d' ' -f1)
size=$(stat -c %s "$out/$name")
version=$(sed -n 's/^version = "\(.*\)"/\1/p' src-tauri/Cargo.toml | head -1)
jq -n --arg version "$version" --arg build "$POPRAWIACZ_BUILD" --argjson built_at "$built_at" \
  --arg sha256 "$sum" --arg url "$name" --argjson size "$size" --arg notes "$notes" \
  '{version:$version, build:$build, built_at:$built_at, sha256:$sha256, url:$url, size:$size, notes:$notes}' \
  > "$out/latest.json.part"
mv "$out/latest.json.part" "$out/latest.json"

# Keep the newest $keep builds.
ls -1t "$out"/PoprawiaczTekstu-*.exe 2>/dev/null | tail -n +"$((keep + 1))" | while read -r old; do rm -f -- "$old"; done

echo "opublikowano $name ($((size / 1024)) KiB, $profile) → $out/latest.json"
