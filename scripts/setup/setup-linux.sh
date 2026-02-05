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

require_sudo() {
  if [ "$(id -u)" -eq 0 ]; then
    echo ""
  elif have_cmd sudo; then
    echo "sudo"
  else
    die "sudo is required to install system packages"
  fi
}

install_debian() {
  local sudo_cmd
  sudo_cmd="$(require_sudo)"
  ${sudo_cmd} apt-get update
  ${sudo_cmd} apt-get install -y \
    build-essential \
    clang \
    cmake \
    ninja-build \
    pkg-config \
    libssl-dev \
    libasound2-dev \
    curl \
    ca-certificates
}

install_fedora() {
  local sudo_cmd
  sudo_cmd="$(require_sudo)"
  ${sudo_cmd} dnf install -y \
    gcc \
    gcc-c++ \
    clang \
    cmake \
    ninja-build \
    pkgconf-pkg-config \
    openssl-devel \
    alsa-lib-devel \
    curl \
    ca-certificates
}

install_arch() {
  local sudo_cmd
  sudo_cmd="$(require_sudo)"
  ${sudo_cmd} pacman -Sy --noconfirm \
    base-devel \
    clang \
    cmake \
    ninja \
    pkgconf \
    openssl \
    alsa-lib \
    curl \
    ca-certificates
}

install_system_deps() {
  if [ ! -r /etc/os-release ]; then
    die "cannot determine distro (missing /etc/os-release)"
  fi

  # shellcheck disable=SC1091
  . /etc/os-release

  case "${ID:-}" in
    debian|ubuntu)
      log "Installing system dependencies (Debian/Ubuntu)"
      install_debian
      return
      ;;
    fedora|rhel|centos|rocky|almalinux)
      log "Installing system dependencies (Fedora/RHEL)"
      install_fedora
      return
      ;;
    arch|manjaro)
      log "Installing system dependencies (Arch)"
      install_arch
      return
      ;;
  esac

  case "${ID_LIKE:-}" in
    *debian*)
      log "Installing system dependencies (Debian-like)"
      install_debian
      ;;
    *rhel*|*fedora*)
      log "Installing system dependencies (RHEL/Fedora-like)"
      install_fedora
      ;;
    *arch*)
      log "Installing system dependencies (Arch-like)"
      install_arch
      ;;
    *)
      die "unsupported distro: ID=${ID:-unknown} ID_LIKE=${ID_LIKE:-unknown}"
      ;;
  esac
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
