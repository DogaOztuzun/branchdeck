#!/usr/bin/env sh
# Branchdeck Installer
# Usage: curl -fsSL https://raw.githubusercontent.com/unquale/Branchdeck/main/scripts/install.sh | sh
#
# Detects platform and architecture, downloads the latest release from GitHub,
# and installs the Branchdeck desktop app + daemon binary.
#
# Options (via environment variables):
#   BRANCHDECK_VERSION  - specific version to install (default: latest)
#   BRANCHDECK_INSTALL_DIR - where to install binaries (default: /usr/local/bin)
#   BRANCHDECK_NO_DESKTOP - set to 1 to skip desktop app, install daemon only

set -eu

REPO="unquale/Branchdeck"
GITHUB_API="https://api.github.com"
INSTALL_DIR="${BRANCHDECK_INSTALL_DIR:-/usr/local/bin}"
NO_DESKTOP="${BRANCHDECK_NO_DESKTOP:-0}"

# --- Helpers ---

info() {
  printf '\033[1;34m%s\033[0m %s\n' "::" "$1"
}

success() {
  printf '\033[1;32m%s\033[0m %s\n' "OK" "$1"
}

error() {
  printf '\033[1;31m%s\033[0m %s\n' "ERROR" "$1" >&2
  exit 1
}

warn() {
  printf '\033[1;33m%s\033[0m %s\n' "WARN" "$1" >&2
}

need_cmd() {
  if ! command -v "$1" > /dev/null 2>&1; then
    error "Required command not found: $1"
  fi
}

# --- Platform detection ---

detect_arch() {
  arch="$(uname -m)"
  case "$arch" in
    x86_64|amd64) echo "amd64" ;;
    aarch64|arm64) echo "arm64" ;;
    *) error "Unsupported architecture: $arch" ;;
  esac
}

detect_os() {
  os="$(uname -s)"
  case "$os" in
    Linux) echo "linux" ;;
    *) error "Unsupported OS: $os. Branchdeck is Linux-only." ;;
  esac
}

detect_package_manager() {
  if command -v dpkg > /dev/null 2>&1; then
    echo "deb"
  elif command -v rpm > /dev/null 2>&1; then
    echo "rpm"
  else
    echo "none"
  fi
}

# --- GitHub API ---

get_latest_version() {
  need_cmd curl
  need_cmd grep
  need_cmd cut

  version=$(curl -fsSL "${GITHUB_API}/repos/${REPO}/releases/latest" 2>/dev/null \
    | grep '"tag_name"' \
    | cut -d'"' -f4)

  if [ -z "$version" ]; then
    error "Failed to fetch latest version from GitHub. Check your network connection."
  fi

  echo "$version"
}

get_download_url() {
  tag="$1"
  pattern="$2"

  need_cmd curl

  url=$(curl -fsSL "${GITHUB_API}/repos/${REPO}/releases/tags/${tag}" 2>/dev/null \
    | grep "browser_download_url" \
    | grep "$pattern" \
    | head -1 \
    | cut -d'"' -f4)

  echo "$url"
}

# --- Installation ---

download_file() {
  url="$1"
  dest="$2"

  info "Downloading $(basename "$dest")..."

  if command -v curl > /dev/null 2>&1; then
    curl -fSL --progress-bar -o "$dest" "$url"
  elif command -v wget > /dev/null 2>&1; then
    wget -q --show-progress -O "$dest" "$url"
  else
    error "Neither curl nor wget found. Install one and retry."
  fi
}

install_deb() {
  deb_file="$1"
  info "Installing .deb package..."

  if [ "$(id -u)" -eq 0 ]; then
    dpkg -i "$deb_file" 2>/dev/null || apt-get install -f -y 2>/dev/null
  else
    sudo dpkg -i "$deb_file" 2>/dev/null || sudo apt-get install -f -y 2>/dev/null
  fi
}

install_rpm() {
  rpm_file="$1"
  info "Installing .rpm package..."

  if [ "$(id -u)" -eq 0 ]; then
    rpm -U "$rpm_file" 2>/dev/null || true
  else
    sudo rpm -U "$rpm_file" 2>/dev/null || true
  fi
}

install_appimage() {
  appimage_file="$1"
  target="${INSTALL_DIR}/branchdeck"

  info "Installing AppImage to ${target}..."

  chmod +x "$appimage_file"

  if [ "$(id -u)" -eq 0 ]; then
    mv "$appimage_file" "$target"
  else
    sudo mv "$appimage_file" "$target"
  fi

  success "AppImage installed to ${target}"
}

install_daemon_binary() {
  tag="$1"
  arch="$2"

  daemon_url=$(get_download_url "$tag" "branchdeck-daemon-linux-${arch}")

  if [ -z "$daemon_url" ]; then
    warn "Daemon binary not found in release. The desktop app includes the daemon."
    return
  fi

  tmpfile=$(mktemp)
  download_file "$daemon_url" "$tmpfile"
  chmod +x "$tmpfile"

  target="${INSTALL_DIR}/branchdeck-daemon"

  if [ "$(id -u)" -eq 0 ]; then
    mv "$tmpfile" "$target"
  else
    sudo mv "$tmpfile" "$target"
  fi

  success "Daemon installed to ${target}"
}

install_desktop_app() {
  tag="$1"
  arch="$2"
  pkg_mgr="$3"
  version_num="${tag#v}"

  tmpdir=$(mktemp -d)
  trap 'rm -rf "$tmpdir"' EXIT

  case "$pkg_mgr" in
    deb)
      deb_url=$(get_download_url "$tag" "branchdeck_${version_num}_${arch}.deb")
      if [ -z "$deb_url" ]; then
        # Try alternate naming patterns
        deb_url=$(get_download_url "$tag" ".deb")
      fi

      if [ -n "$deb_url" ]; then
        deb_file="${tmpdir}/branchdeck.deb"
        download_file "$deb_url" "$deb_file"
        install_deb "$deb_file"
        success "Desktop app installed via .deb package"
        return
      fi

      warn ".deb package not found, falling back to AppImage"
      ;;
    rpm)
      rpm_url=$(get_download_url "$tag" ".rpm")

      if [ -n "$rpm_url" ]; then
        rpm_file="${tmpdir}/branchdeck.rpm"
        download_file "$rpm_url" "$rpm_file"
        install_rpm "$rpm_file"
        success "Desktop app installed via .rpm package"
        return
      fi

      warn ".rpm package not found, falling back to AppImage"
      ;;
  esac

  # Fallback: AppImage
  appimage_url=$(get_download_url "$tag" ".AppImage")

  if [ -z "$appimage_url" ]; then
    error "No installable package found for this release. Check ${GITHUB_API}/repos/${REPO}/releases/tags/${tag}"
  fi

  appimage_file="${tmpdir}/branchdeck.AppImage"
  download_file "$appimage_url" "$appimage_file"
  install_appimage "$appimage_file"
  # Override trap since we moved the file
  trap '' EXIT
}

# --- Main ---

main() {
  info "Branchdeck Installer"
  echo ""

  os=$(detect_os)
  arch=$(detect_arch)
  pkg_mgr=$(detect_package_manager)

  info "Platform: ${os}/${arch} (package manager: ${pkg_mgr})"

  if [ -n "${BRANCHDECK_VERSION:-}" ]; then
    tag="v${BRANCHDECK_VERSION#v}"
    info "Installing specified version: ${tag}"
  else
    info "Fetching latest version..."
    tag=$(get_latest_version)
    info "Latest version: ${tag}"
  fi

  # Install desktop app (unless --daemon-only)
  if [ "$NO_DESKTOP" != "1" ]; then
    install_desktop_app "$tag" "$arch" "$pkg_mgr"
  fi

  # Install standalone daemon binary
  install_daemon_binary "$tag" "$arch"

  echo ""
  success "Branchdeck ${tag} installed successfully!"
  echo ""
  info "Run 'branchdeck' to launch the desktop app"
  info "Run 'branchdeck-daemon serve' to start the daemon standalone"
  echo ""
}

main
