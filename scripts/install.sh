#!/bin/sh
# dmaf engine installer (macOS / Linux)
# Official entry: https://dmaf.aicoffe.com/install.sh
# Contract: fixed install location, re-running the same command upgrades in place;
# PATH is appended to the shell rc; the downloaded sha256 is always verified (fail closed).
set -eu

MARKET_BASE="${DMAF_MARKET_URL:-https://dmaf.aicoffe.com}"
REPO_SLUG="aicoffe/dmaf-engine"
INSTALL_DIR="${DMAF_INSTALL_DIR:-$HOME/.local/bin}"

VERSION=""
SOURCE="auto"   # auto | market | github
NO_PATH=0

usage() {
  cat <<EOF
Usage: install.sh [--version <v>] [--source auto|market|github] [--no-path]
  --version <v>   install a specific version (e.g. 0.7.0 or v0.7.0); default: latest
  --source        download source; default: market first, GitHub fallback
  --no-path       do not touch PATH / shell rc
Environment: DMAF_MARKET_URL, DMAF_INSTALL_DIR
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --version) VERSION="${2:-}"; shift 2 ;;
    --version=*) VERSION="${1#*=}"; shift ;;
    --source) SOURCE="${2:-}"; shift 2 ;;
    --source=*) SOURCE="${1#*=}"; shift ;;
    --no-path) NO_PATH=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'install.sh: unknown argument: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

# ---- platform detection -> release target triple ----
OS_NAME=$(uname -s)
ARCH_NAME=$(uname -m)
case "$OS_NAME" in
  MINGW*|MSYS*|CYGWIN*|Windows_NT)
    printf 'install.sh: Windows detected. Use the PowerShell installer instead:\n  irm https://dmaf.aicoffe.com/install.ps1 | iex\n' >&2
    exit 2
    ;;
esac
case "$OS_NAME:$ARCH_NAME" in
  Darwin:arm64)   TARGET="aarch64-apple-darwin" ;;
  Darwin:x86_64)  TARGET="x86_64-apple-darwin" ;;
  Linux:x86_64)   TARGET="x86_64-unknown-linux-musl" ;;
  Linux:aarch64)  TARGET="aarch64-unknown-linux-musl" ;;
  *)
    printf 'install.sh: unsupported platform: %s %s\n' "$OS_NAME" "$ARCH_NAME" >&2
    exit 2
    ;;
esac

# ---- fetch helper (curl first, wget fallback) ----
fetch() {
  _url="$1"; _out="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fSL --retry 3 --connect-timeout 15 -o "$_out" "$_url"
  elif command -v wget >/dev/null 2>&1; then
    wget -q -O "$_out" "$_url"
  else
    printf 'install.sh: neither curl nor wget is available\n' >&2
    return 1
  fi
}

# HEAD probe (for auto-source fallback; no full download)
probe() {
  _url="$1"
  if command -v curl >/dev/null 2>&1; then
    curl -fsIL --connect-timeout 15 "$_url" >/dev/null
  else
    wget --spider -q "$_url"
  fi
}

# ---- version resolution ----
VERSION=$(printf '%s' "$VERSION" | sed 's/^v//')
if [ -z "$VERSION" ]; then
  _resolved=""
  if [ "$SOURCE" != "github" ]; then
    if fetch "$MARKET_BASE/api/engine/latest" /tmp/dmaf-manifest.$$ 2>/dev/null; then
      _resolved=$(tr -d '\n' < /tmp/dmaf-manifest.$$ | sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)
    fi
    rm -f /tmp/dmaf-manifest.$$
  fi
  if [ -z "$_resolved" ] && [ "$SOURCE" != "market" ]; then
    fetch "https://api.github.com/repos/$REPO_SLUG/releases/latest" /tmp/dmaf-rel.$$
    _resolved=$(tr -d '\n' < /tmp/dmaf-rel.$$ | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)
    rm -f /tmp/dmaf-rel.$$
  fi
  if [ -z "$_resolved" ]; then
    printf 'install.sh: cannot resolve latest version (source=%s)\n' "$SOURCE" >&2
    exit 3
  fi
  VERSION=$(printf '%s' "$_resolved" | sed 's/^v//')
fi

# Naming freeze (actual dist 0.34 artifacts): archives are dmaf-engine-TARGET.tar.xz / .zip
# (no version segment; the version lives in the release tag); verification = per-asset
# same-named .sha256 (content "HASH  FILENAME"); the market mirror must byte-for-byte
# mirror all Release assets (including sha256.sum).
ASSET="dmaf-engine-${TARGET}.tar.xz"
case "$SOURCE" in
  market) BASE_URL="$MARKET_BASE/engine/v$VERSION" ;;
  github) BASE_URL="https://github.com/$REPO_SLUG/releases/download/v$VERSION" ;;
  *)
    BASE_URL="$MARKET_BASE/engine/v$VERSION"
    if ! probe "$BASE_URL/$ASSET"; then
      BASE_URL="https://github.com/$REPO_SLUG/releases/download/v$VERSION"
    fi
    ;;
esac

# ---- download + sha256 verify (fail closed) ----
TMPDIR_WORK=$(mktemp -d)
trap 'rm -rf "$TMPDIR_WORK"' EXIT INT TERM

printf '==> downloading %s (v%s, %s)\n' "$ASSET" "$VERSION" "$SOURCE"
fetch "$BASE_URL/$ASSET" "$TMPDIR_WORK/$ASSET"
fetch "$BASE_URL/$ASSET.sha256" "$TMPDIR_WORK/$ASSET.sha256"

WANT_HASH=$(cut -d' ' -f1 "$TMPDIR_WORK/$ASSET.sha256" | head -1)
if [ -z "$WANT_HASH" ]; then
  printf 'install.sh: %s.sha256 missing or malformed (fail closed)\n' "$ASSET" >&2
  exit 4
fi
if command -v sha256sum >/dev/null 2>&1; then
  GOT_HASH=$(sha256sum < "$TMPDIR_WORK/$ASSET" | cut -d' ' -f1)
elif command -v shasum >/dev/null 2>&1; then
  GOT_HASH=$(shasum -a 256 < "$TMPDIR_WORK/$ASSET" | cut -d' ' -f1)
else
  GOT_HASH=$(openssl dgst -sha256 -r < "$TMPDIR_WORK/$ASSET" | cut -d' ' -f1)
fi
if [ "$GOT_HASH" != "$WANT_HASH" ]; then
  printf 'install.sh: sha256 mismatch (fail closed)\n  want %s\n  got  %s\n' "$WANT_HASH" "$GOT_HASH" >&2
  exit 4
fi

# ---- install (fixed location; re-run = upgrade, idempotent) ----
tar -xJf "$TMPDIR_WORK/$ASSET" -C "$TMPDIR_WORK" || {
  printf 'install.sh: tar -xJf failed (is xz installed?)\n' >&2
  exit 4
}
if [ ! -f "$TMPDIR_WORK/dmaf" ]; then
  printf 'install.sh: archive does not contain ./dmaf\n' >&2
  exit 4
fi
mkdir -p "$INSTALL_DIR"
mv -f "$TMPDIR_WORK/dmaf" "$INSTALL_DIR/dmaf"
chmod +x "$INSTALL_DIR/dmaf"

# ---- PATH append (only when missing; auto rc per D9) ----
if [ "$NO_PATH" -ne 1 ]; then
  case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
      _rc=""
      case "${SHELL:-}" in
        *zsh)  _rc="$HOME/.zshrc" ;;
        *bash) [ -f "$HOME/.bashrc" ] && _rc="$HOME/.bashrc" || _rc="$HOME/.profile" ;;
        *)     _rc="$HOME/.profile" ;;
      esac
      {
        printf '\n'
        printf '# added by dmaf installer\n'
        printf 'export PATH="%s:$PATH"\n' "$INSTALL_DIR"
      } >> "$_rc"
      printf '==> PATH updated: appended %s to %s (open a new terminal or: source %s)\n' "$INSTALL_DIR" "$_rc" "$_rc"
      ;;
  esac
fi

printf '==> installed: %s\n' "$INSTALL_DIR/dmaf"
"$INSTALL_DIR/dmaf" --version || true
printf '==> next: run "dmaf doctor" to verify the environment\n'
printf '    workflow project: dmaf add https://dmaf.aicoffe.com/aicoffe/dmaf-sdlc\n'
