set -euo pipefail

FOIL_REPO="${FOIL_REPO:-francisco/foil-core}"
FOIL_VERSION="${FOIL_VERSION:-latest}"
FOIL_INSTALL_DIR="${FOIL_INSTALL_DIR:-/usr/local/bin}"
FOIL_APP_DIR="${FOIL_APP_DIR:-/Applications}"

detect_os() {
  case "$(uname -s)" in
    Darwin*) echo "macos" ;;
    Linux*)  echo "linux" ;;
    *)       echo "unsupported" ;;
  esac
}

resolve_download_url() {
  local asset_name="$1"
  if [ "$FOIL_VERSION" = "latest" ]; then
    echo "https://github.com/${FOIL_REPO}/releases/latest/download/${asset_name}"
  else
    echo "https://github.com/${FOIL_REPO}/releases/download/${FOIL_VERSION}/${asset_name}"
  fi
}

run_with_privilege() {
  if [ -w "$1" ]; then
    shift
    "$@"
  elif command -v sudo >/dev/null 2>&1; then
    shift
    sudo "$@"
  else
    shift
    "$@"
  fi
}

install_macos_from_dmg() {
  local dmg_path="$1"
  local mount_point
  mount_point="$(mktemp -d /tmp/foil-dmg.XXXXXX)"

  echo "==> Mounting ${dmg_path}..."
  hdiutil attach "$dmg_path" -mountpoint "$mount_point" -nobrowse -quiet

  if [ -d "${mount_point}/Foil.app" ]; then
    echo "==> Installing Foil.app to ${FOIL_APP_DIR}/Foil.app..."
    run_with_privilege "$FOIL_APP_DIR" rm -rf "${FOIL_APP_DIR}/Foil.app"
    run_with_privilege "$FOIL_APP_DIR" cp -R "${mount_point}/Foil.app" "${FOIL_APP_DIR}/Foil.app"
    run_with_privilege "$FOIL_APP_DIR" xattr -cr "${FOIL_APP_DIR}/Foil.app" 2>/dev/null || true

    echo "==> Linking foil CLI to ${FOIL_INSTALL_DIR}/foil..."
    run_with_privilege "$(dirname "$FOIL_INSTALL_DIR")" mkdir -p "$FOIL_INSTALL_DIR"
    run_with_privilege "$FOIL_INSTALL_DIR" ln -sf "${FOIL_APP_DIR}/Foil.app/Contents/MacOS/foil" "${FOIL_INSTALL_DIR}/foil"
  fi

  hdiutil detach "$mount_point" -quiet || true
  rmdir "$mount_point" 2>/dev/null || true
}

install_macos_local_binary() {
  local bin_path="$1"
  local app_contents="${FOIL_APP_DIR}/Foil.app/Contents"

  echo "==> Creating ${FOIL_APP_DIR}/Foil.app from local binary ${bin_path}..."
  run_with_privilege "$FOIL_APP_DIR" mkdir -p "${app_contents}/MacOS" "${app_contents}/Resources"
  run_with_privilege "$FOIL_APP_DIR" cp "$bin_path" "${app_contents}/MacOS/foil"
  run_with_privilege "$FOIL_APP_DIR" chmod +x "${app_contents}/MacOS/foil"

  local tmp_plist
  tmp_plist="$(mktemp /tmp/foil-plist.XXXXXX)"
  cat << 'PLIST' > "$tmp_plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>foil</string>
  <key>CFBundleIdentifier</key>
  <string>dev.foil.player</string>
  <key>CFBundleName</key>
  <string>Foil</string>
  <key>CFBundleDisplayName</key>
  <string>Foil Player</string>
  <key>CFBundleVersion</key>
  <string>0.1.0</string>
  <key>CFBundleShortVersionString</key>
  <string>0.1.0</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>CFBundleDocumentTypes</key>
  <array>
    <dict>
      <key>CFBundleTypeName</key>
      <string>Foil Presentation</string>
      <key>CFBundleTypeRole</key>
      <string>Viewer</string>
      <key>CFBundleTypeExtensions</key>
      <array>
        <string>foil</string>
      </array>
    </dict>
  </array>
</dict>
</plist>
PLIST

  run_with_privilege "$FOIL_APP_DIR" cp "$tmp_plist" "${app_contents}/Info.plist"
  rm -f "$tmp_plist"
  run_with_privilege "$FOIL_APP_DIR" xattr -cr "${FOIL_APP_DIR}/Foil.app" 2>/dev/null || true

  echo "==> Linking foil CLI to ${FOIL_INSTALL_DIR}/foil..."
  run_with_privilege "$(dirname "$FOIL_INSTALL_DIR")" mkdir -p "$FOIL_INSTALL_DIR"
  run_with_privilege "$FOIL_INSTALL_DIR" ln -sf "${app_contents}/MacOS/foil" "${FOIL_INSTALL_DIR}/foil"
}

install_macos() {
  if [ -f "target/macos-dmg/foil-macos-universal.dmg" ]; then
    install_macos_from_dmg "target/macos-dmg/foil-macos-universal.dmg"
  elif [ -f "target/release/foil" ]; then
    install_macos_local_binary "target/release/foil"
  else
    local tmp_dmg
    tmp_dmg="$(mktemp /tmp/foil-macos.XXXXXX.dmg)"
    local url
    url="$(resolve_download_url "foil-macos-universal.dmg")"
    echo "==> Downloading ${url}..."
    curl -fsSL "$url" -o "$tmp_dmg"
    install_macos_from_dmg "$tmp_dmg"
    rm -f "$tmp_dmg"
  fi
  echo "==> macOS installation complete: ${FOIL_APP_DIR}/Foil.app & ${FOIL_INSTALL_DIR}/foil"
}

install_linux() {
  local tmp_bin=""
  local source_bin=""

  if [ -f "target/linux-bin/foil-linux-x86_64" ]; then
    source_bin="target/linux-bin/foil-linux-x86_64"
  elif [ -f "target/release/foil" ]; then
    source_bin="target/release/foil"
  else
    tmp_bin="$(mktemp /tmp/foil-linux.XXXXXX)"
    local url
    url="$(resolve_download_url "foil-linux-x86_64")"
    echo "==> Downloading ${url}..."
    curl -fsSL "$url" -o "$tmp_bin"
    source_bin="$tmp_bin"
  fi

  echo "==> Installing foil binary to ${FOIL_INSTALL_DIR}/foil..."
  run_with_privilege "$(dirname "$FOIL_INSTALL_DIR")" mkdir -p "$FOIL_INSTALL_DIR"
  run_with_privilege "$FOIL_INSTALL_DIR" cp "$source_bin" "${FOIL_INSTALL_DIR}/foil"
  run_with_privilege "$FOIL_INSTALL_DIR" chmod 755 "${FOIL_INSTALL_DIR}/foil"

  if [ -n "$tmp_bin" ]; then
    rm -f "$tmp_bin"
  fi
  echo "==> Linux installation complete: ${FOIL_INSTALL_DIR}/foil"
}

main() {
  local os_type
  os_type="$(detect_os)"
  case "$os_type" in
    macos) install_macos ;;
    linux) install_linux ;;
    *)
      echo "Unsupported OS: $(uname -s). Please use the .msi installer on Windows." >&2
      exit 1
      ;;
  esac
}

main "$@"
