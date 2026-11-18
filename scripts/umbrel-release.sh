#!/usr/bin/env bash
# Baut nrzPlan fuer umbrelOS (Linux amd64 + arm64), legt die fertigen Dateien auf den Zweig
# `umbrel` und pusht den Tag `umbrel-<version>`. nrzGit traegt die Version dann per nrzStore.json
# selbst im nrzStore ein (RELEASE in hooks/pre-start, version, releaseNotes, Store-Version).
#
#   ./scripts/umbrel-release.sh 1.0.1 "Kurze Beschreibung"
#
# Voraussetzungen: dx (dioxus-cli 0.7), pnpm, zig + cargo-zigbuild (Linker fuer Linux), Rust-Targets
# x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu und wasm32-unknown-unknown.
set -euo pipefail

VERSION="${1:?Version fehlt, z.B. 1.0.1}"
NOTES="${2:-nrzPlan ${VERSION}}"
TAG="umbrel-${VERSION}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

if [ -n "$(git status --porcelain)" ]; then
  echo "Arbeitsverzeichnis nicht sauber – erst committen." >&2
  exit 1
fi
if git rev-parse -q --verify "refs/tags/${TAG}" >/dev/null; then
  echo "Tag ${TAG} gibt es schon." >&2
  exit 1
fi

# dioxus-cli finden (`dx` im PATH kann auch Denos dx sein).
DX="${DX:-}"
for c in "${DX}" "$(command -v dx || true)" "${HOME}/.cargo/bin/dx"; do
  if [ -n "${c}" ] && "${c}" --version 2>/dev/null | grep -qi dioxus; then DX="${c}"; break; fi
  DX=""
done
[ -n "${DX}" ] || { echo "dioxus-cli (dx) nicht gefunden – cargo binstall dioxus-cli" >&2; exit 1; }

# zig-Linker-Wrapper von cargo-zigbuild finden (glibc 2.31 laeuft auf jedem aktuellen Debian).
cargo zigbuild --help >/dev/null 2>&1 || { echo "cargo-zigbuild fehlt" >&2; exit 1; }
WRAP="$(find "${HOME}/Library/Caches/cargo-zigbuild" "${HOME}/.cache/cargo-zigbuild" -name 'zigcc-x86_64-unknown-linux-gnu.2.31-*.sh' 2>/dev/null | head -1 || true)"
if [ -z "${WRAP}" ]; then
  # Einmal anstossen, damit cargo-zigbuild die Wrapper anlegt.
  cargo zigbuild --release --features server --target x86_64-unknown-linux-gnu.2.31 >/dev/null 2>&1 || true
  cargo zigbuild --release --features server --target aarch64-unknown-linux-gnu.2.31 >/dev/null 2>&1 || true
  WRAP="$(find "${HOME}/Library/Caches/cargo-zigbuild" "${HOME}/.cache/cargo-zigbuild" -name 'zigcc-x86_64-unknown-linux-gnu.2.31-*.sh' 2>/dev/null | head -1)"
fi
WDIR="$(dirname "${WRAP}")"

OUT="$(mktemp -d)"
LISTS="$(mktemp -d)"
trap 'rm -rf "${OUT}" "${LISTS}"' EXIT
WEB="target/dx/nrzplan/release/web"

build() { # <rust-target> <arch>
  local triple="$1" arch="$2" up cc
  up="$(echo "${triple}" | tr 'a-z-' 'A-Z_')"
  cc="$(ls "${WDIR}"/zigcc-"${triple}".2.31-*.sh | head -1)"
  echo "==> Baue ${arch} (${triple})"
  rm -rf target/dx/nrzplan/release
  env "CARGO_TARGET_${up}_LINKER=${cc}" "CC_${triple//-/_}=${cc}" "CC_${triple}=${cc}" \
    "AR_${triple//-/_}=${WDIR}/ar" "AR_${triple}=${WDIR}/ar" \
    "${DX}" bundle --platform web --fullstack --release @server --target "${triple}" --platform server --features server
  # Ohne `--platform server` uebernimmt dx beim eigenen @server-Target das Client-Feature `web`
  # (dioxus-web) in den Server: der liefert dann keine Hydration-Daten fuer use_server_future
  # und jede Seite bricht im Browser mit "Cannot read properties of undefined" ab.
  if ! strings "${WEB}/server" | grep -c "calendar.ics" >/dev/null; then
    echo "Server-Binary fuer ${arch} enthaelt keinen Server-Teil – Abbruch." >&2
    exit 1
  fi
  if strings "${WEB}/server" | grep -qE "dioxus-web-0\.|sledgehammer"; then
    echo "Server-Binary fuer ${arch} enthaelt dioxus-web – Abbruch." >&2
    exit 1
  fi
  # Server und Oberflaeche muessen dieselben Asset-Namen kennen.
  local css hash
  css="$(cd "${WEB}/public/assets" && ls tailwind-*.css | head -1)"
  hash="${css#tailwind-}"; hash="${hash%.css}"
  if ! strings "${WEB}/server" | grep -c "${hash}" >/dev/null; then
    echo "Server-Binary fuer ${arch} kennt ${css} nicht – Abbruch." >&2
    exit 1
  fi
  (cd "${WEB}/public" && find . -type f | sort) > "${LISTS}/${arch}.txt"
  gzip -9 -c "${WEB}/server" > "${OUT}/nrzplan-${arch}.gz"
}

echo "==> Tailwind"
# pnpm 10 meldet ignorierte Build-Skripte (@parcel/watcher) als Fehler; fuer css:build unnoetig.
pnpm install --silent >/dev/null 2>&1 || true
pnpm css:build >/dev/null
build x86_64-unknown-linux-gnu amd64
COPYFILE_DISABLE=1 tar --no-xattrs -czf "${OUT}/public.tar.gz" -C "${WEB}/public" .
build aarch64-unknown-linux-gnu arm64
if ! diff -q "${LISTS}/amd64.txt" "${LISTS}/arm64.txt" >/dev/null; then
  echo "Oberflaeche von amd64 und arm64 unterscheidet sich – Abbruch." >&2
  exit 1
fi
(cd "${OUT}" && shasum -a 256 nrzplan-amd64.gz nrzplan-arm64.gz public.tar.gz > SHA256SUMS)

# Zweig `umbrel` in einem eigenen Worktree aktualisieren (nur fertige Dateien).
WT="$(mktemp -d)"
git fetch origin umbrel 2>/dev/null || true
if git rev-parse -q --verify origin/umbrel >/dev/null; then
  git worktree add -q "${WT}" -B umbrel origin/umbrel
else
  git worktree add -q --detach "${WT}"
  (cd "${WT}" && git checkout -q --orphan umbrel && git rm -rqf .)
fi
cp "${OUT}"/* "${WT}/"
git show HEAD:scripts/umbrel-README.md > "${WT}/README.md"
(
  cd "${WT}"
  git add -A
  git commit -qm "nrzPlan ${VERSION}: Umbrel-Paket"
  git tag -a "${TAG}" -m "${NOTES}"
  git push -q origin umbrel
  git push -q origin "${TAG}"
)
git worktree remove --force "${WT}"
echo "==> ${TAG} veroeffentlicht. nrzGit aktualisiert den nrzStore."
