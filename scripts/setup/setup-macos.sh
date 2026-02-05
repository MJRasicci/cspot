#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
export PATH="${HOME}/.cargo/bin:${PATH}"

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

install_system_deps() {
  install_xcode_clt
  if ! have_cmd clang; then
    warn "clang not found; ensure Xcode Command Line Tools are installed"
  fi

  install_homebrew
  enable_brew_shellenv

  log "Installing system dependencies via Homebrew"
  brew install cmake ninja pkg-config
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
  version="${CSPOT_CBINDGEN_VERSION:-0.27.0}"
  log "Installing cbindgen ${version}"
  cargo install cbindgen --version "${version}"
}

update_submodules() {
  if have_cmd git && [ -d "${REPO_ROOT}/.git" ]; then
    log "Updating git submodules"
    git -C "${REPO_ROOT}" submodule update --init --recursive
  fi
}

main() {
  install_system_deps
  install_rust
  install_rust_components
  install_cbindgen
  update_submodules

  log "Setup complete. Open a new shell to pick up PATH updates if needed."
}

main "$@"
