#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
export PATH="${HOME}/.cargo/bin:${PATH}"

ENABLE_ANDROID=0
ANDROID_API_LEVEL="${CSPOT_ANDROID_API_LEVEL:-26}"
ANDROID_BUILD_TOOLS_VERSION="${CSPOT_ANDROID_BUILD_TOOLS_VERSION:-34.0.0}"
ANDROID_NDK_VERSION="${CSPOT_ANDROID_NDK_VERSION:-27.2.12479018}"
ANDROID_CMDLINE_TOOLS_VERSION="${CSPOT_ANDROID_CMDLINE_TOOLS_VERSION:-13114758}"

log() {
  printf "[cspot-setup] %s\n" "$*"
}

warn() {
  printf "[cspot-setup] WARN: %s\n" "$*" >&2
}

die() {
  printf "[cspot-setup] ERROR: %s\n" "$*" >&2
  exit 1
}

have_cmd() {
  command -v "$1" >/dev/null 2>&1
}

usage() {
  cat <<'EOF'
Usage: ./scripts/setup.sh [--android]

Options:
  --android   Install Android SDK/NDK tooling and Rust Android targets.
  -h, --help  Show this help text.
EOF
}

parse_args() {
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --android)
        ENABLE_ANDROID=1
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        die "unknown argument: $1"
        ;;
    esac
    shift
  done
}

enable_brew_shellenv() {
  if [ -x /opt/homebrew/bin/brew ]; then
    eval "$(/opt/homebrew/bin/brew shellenv)"
  elif [ -x /usr/local/bin/brew ]; then
    eval "$(/usr/local/bin/brew shellenv)"
  fi
}

install_xcode_clt() {
  if xcode-select -p >/dev/null 2>&1; then
    return
  fi

  log "Installing Xcode Command Line Tools"
  xcode-select --install || true
  warn "Re-run this script after the installer completes"
  return 1
}

install_homebrew() {
  if have_cmd brew; then
    return
  fi

  log "Installing Homebrew"
  /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
  enable_brew_shellenv
}

install_brew_package_if_missing() {
  local package
  package="$1"
  if brew list --formula --versions "${package}" >/dev/null 2>&1; then
    log "${package} already installed"
    return
  fi
  brew install "${package}"
}

install_system_deps() {
  install_xcode_clt
  if ! have_cmd clang; then
    warn "clang not found; ensure Xcode Command Line Tools are installed"
  fi

  install_homebrew
  enable_brew_shellenv

  log "Installing system dependencies via Homebrew"
  local packages=(cmake ninja pkg-config)
  if [ "${ENABLE_ANDROID}" -eq 1 ]; then
    packages+=(openjdk@17)
  fi
  local package
  for package in "${packages[@]}"; do
    install_brew_package_if_missing "${package}"
  done
}

install_rust() {
  if have_cmd rustc; then
    log "Rust already installed"
    return
  fi

  log "Installing rustup"
  curl https://sh.rustup.rs -sSf | sh -s -- -y

  if [ -f "${HOME}/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "${HOME}/.cargo/env"
  fi
}

install_rust_components() {
  if have_cmd rustup; then
    rustup component add rustfmt clippy
  fi
}

install_cbindgen() {
  if have_cmd cbindgen; then
    log "cbindgen already installed"
    return
  fi

  if ! have_cmd cargo; then
    die "cargo not found; rustup installation may have failed"
  fi

  local version
  version="${CSPOT_CBINDGEN_VERSION:-0.29.2}"
  log "Installing cbindgen ${version}"
  cargo install cbindgen --version "${version}"
}

update_submodules() {
  if have_cmd git && [ -d "${REPO_ROOT}/.git" ]; then
    log "Updating git submodules"
    git -C "${REPO_ROOT}" submodule update --init --recursive
  fi
}

resolve_java_home() {
  if [ -n "${JAVA_HOME:-}" ] && [ -d "${JAVA_HOME}" ]; then
    return 0
  fi

  if [ -x /usr/libexec/java_home ]; then
    local java_home
    java_home="$(/usr/libexec/java_home -v 17 2>/dev/null || true)"
    if [ -n "${java_home}" ]; then
      export JAVA_HOME="${java_home}"
      return 0
    fi
  fi

  if have_cmd brew; then
    local brew_openjdk
    brew_openjdk="$(brew --prefix openjdk@17 2>/dev/null || true)"
    if [ -n "${brew_openjdk}" ] && [ -d "${brew_openjdk}" ]; then
      export JAVA_HOME="${brew_openjdk}/libexec/openjdk.jdk/Contents/Home"
      return 0
    fi
  fi

  die "unable to determine JAVA_HOME for Android sdkmanager"
}

prepare_android_environment() {
  if [ -z "${ANDROID_SDK_ROOT:-}" ]; then
    export ANDROID_SDK_ROOT="${HOME}/Library/Android/sdk"
  fi
  export ANDROID_HOME="${ANDROID_SDK_ROOT}"

  if [ -n "${ANDROID_NDK_HOME:-}" ]; then
    export ANDROID_NDK_ROOT="${ANDROID_NDK_HOME}"
  elif [ -n "${ANDROID_NDK_ROOT:-}" ]; then
    export ANDROID_NDK_HOME="${ANDROID_NDK_ROOT}"
  fi

  mkdir -p "${ANDROID_SDK_ROOT}/cmdline-tools"
}

install_android_cmdline_tools() {
  local sdkmanager
  sdkmanager="${ANDROID_SDK_ROOT}/cmdline-tools/latest/bin/sdkmanager"
  if [ -x "${sdkmanager}" ]; then
    return
  fi

  local archive_url
  archive_url="${CSPOT_ANDROID_CMDLINE_TOOLS_URL:-https://dl.google.com/android/repository/commandlinetools-mac-${ANDROID_CMDLINE_TOOLS_VERSION}_latest.zip}"
  local tmp_dir
  tmp_dir="$(mktemp -d)"

  log "Installing Android command-line tools"
  curl -fsSL "${archive_url}" -o "${tmp_dir}/commandlinetools.zip"
  unzip -q "${tmp_dir}/commandlinetools.zip" -d "${tmp_dir}"

  rm -rf "${ANDROID_SDK_ROOT}/cmdline-tools/latest"
  mv "${tmp_dir}/cmdline-tools" "${ANDROID_SDK_ROOT}/cmdline-tools/latest"
  rm -rf "${tmp_dir}"
}

install_android_sdk_components() {
  local sdkmanager
  sdkmanager="${ANDROID_SDK_ROOT}/cmdline-tools/latest/bin/sdkmanager"

  resolve_java_home
  export PATH="${ANDROID_SDK_ROOT}/cmdline-tools/latest/bin:${ANDROID_SDK_ROOT}/platform-tools:${PATH}"

  log "Accepting Android SDK licenses"
  yes | "${sdkmanager}" --sdk_root="${ANDROID_SDK_ROOT}" --licenses >/dev/null || true

  log "Installing Android SDK platform, build-tools, and NDK"
  "${sdkmanager}" --sdk_root="${ANDROID_SDK_ROOT}" \
    "platform-tools" \
    "platforms;android-${ANDROID_API_LEVEL}" \
    "build-tools;${ANDROID_BUILD_TOOLS_VERSION}" \
    "ndk;${ANDROID_NDK_VERSION}"

  export ANDROID_NDK_HOME="${ANDROID_SDK_ROOT}/ndk/${ANDROID_NDK_VERSION}"
  export ANDROID_NDK_ROOT="${ANDROID_NDK_HOME}"
  [ -d "${ANDROID_NDK_HOME}" ] || die "failed to install Android NDK ${ANDROID_NDK_VERSION}"
}

install_android_rust_targets() {
  if ! have_cmd rustup; then
    die "rustup is required to add Android Rust targets"
  fi

  log "Installing Rust Android targets"
  rustup target add \
    aarch64-linux-android \
    armv7-linux-androideabi \
    i686-linux-android \
    x86_64-linux-android
}

install_android_toolchain() {
  prepare_android_environment
  install_android_cmdline_tools
  install_android_sdk_components
  install_android_rust_targets

  log "Android SDK root: ${ANDROID_SDK_ROOT}"
  log "Android NDK home: ${ANDROID_NDK_HOME}"
  warn "Persist ANDROID_SDK_ROOT and ANDROID_NDK_HOME in your shell profile for future sessions."
}

main() {
  parse_args "$@"
  install_system_deps
  install_rust
  install_rust_components
  install_cbindgen
  if [ "${ENABLE_ANDROID}" -eq 1 ]; then
    install_android_toolchain
  fi
  update_submodules

  log "Setup complete. Open a new shell to pick up PATH updates if needed."
}

main "$@"
