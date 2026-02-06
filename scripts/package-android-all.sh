#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

PACK_DIR="${REPO_ROOT}/artifacts/pack"
STAGE_NAME="cspot-android-all-release"
STAGE_DIR="${PACK_DIR}/${STAGE_NAME}"
ARCHIVE_PATH="${PACK_DIR}/${STAGE_NAME}.tar.gz"

log() {
  printf "[cspot-android-package] %s\n" "$*"
}

die() {
  printf "[cspot-android-package] ERROR: %s\n" "$*" >&2
  exit 1
}

require_android_env() {
  if [ -z "${ANDROID_NDK_HOME:-}" ] && [ -z "${ANDROID_NDK_ROOT:-}" ]; then
    die "ANDROID_NDK_HOME or ANDROID_NDK_ROOT must be set. Run ./scripts/setup.sh --android first."
  fi

  if [ -z "${ANDROID_SDK_ROOT:-}" ] && [ -z "${ANDROID_HOME:-}" ]; then
    log "ANDROID_SDK_ROOT is not set; continuing with existing toolchain paths."
  fi
}

run_android_workflows() {
  local preset
  local presets=(
    android-x86-release
    android-x86_64-release
    android-armeabi-v7a-release
    android-arm64-v8a-release
  )

  for preset in "${presets[@]}"; do
    log "Running ${preset}"
    cmake --workflow --preset "${preset}"
  done
}

copy_abi_artifacts() {
  local abi="$1"
  local triple="$2"
  local source_dir="${REPO_ROOT}/artifacts/target/${triple}/release"
  local dest_dir="${STAGE_DIR}/lib/${abi}"

  [ -f "${source_dir}/libcspot.so" ] || die "missing ${source_dir}/libcspot.so"
  [ -f "${source_dir}/libcspot.a" ] || die "missing ${source_dir}/libcspot.a"

  mkdir -p "${dest_dir}"
  cp "${source_dir}/libcspot.so" "${dest_dir}/libcspot.so"
  cp "${source_dir}/libcspot.a" "${dest_dir}/libcspot.a"
}

copy_android_client_apk() {
  local abi="$1"
  local source_apk="${REPO_ROOT}/artifacts/android-client/${abi}/android-client-${abi}-release.apk"
  local dest_dir="${STAGE_DIR}/samples/android-client/apk/${abi}"

  [ -f "${source_apk}" ] || die "missing ${source_apk}"
  mkdir -p "${dest_dir}"
  cp "${source_apk}" "${dest_dir}/android-client-${abi}-release.apk"
}

copy_android_client_sources() {
  local source_dir="${REPO_ROOT}/samples/android-client"
  local dest_dir="${STAGE_DIR}/samples/android-client/source"

  [ -d "${source_dir}" ] || die "missing ${source_dir}"
  mkdir -p "${dest_dir}"

  tar -C "${source_dir}" \
    --exclude './.gradle' \
    --exclude './.idea' \
    --exclude './.cxx' \
    --exclude './build' \
    --exclude './app/build' \
    --exclude './app/.cxx' \
    --exclude './local.properties' \
    -cf - . | tar -C "${dest_dir}" -xf -
}

create_consolidated_archive() {
  mkdir -p "${PACK_DIR}"
  rm -rf "${STAGE_DIR}"
  mkdir -p "${STAGE_DIR}/include"

  [ -f "${REPO_ROOT}/c-bindings/include/cspot.h" ] || die "missing generated header c-bindings/include/cspot.h"
  cp "${REPO_ROOT}/c-bindings/include/cspot.h" "${STAGE_DIR}/include/cspot.h"

  copy_abi_artifacts "x86" "i686-linux-android"
  copy_abi_artifacts "x86_64" "x86_64-linux-android"
  copy_abi_artifacts "armeabi-v7a" "armv7-linux-androideabi"
  copy_abi_artifacts "arm64-v8a" "aarch64-linux-android"

  copy_android_client_apk "x86"
  copy_android_client_apk "x86_64"
  copy_android_client_apk "armeabi-v7a"
  copy_android_client_apk "arm64-v8a"
  copy_android_client_sources

  rm -f "${ARCHIVE_PATH}"
  tar -C "${PACK_DIR}" -czf "${ARCHIVE_PATH}" "${STAGE_NAME}"
  log "Wrote ${ARCHIVE_PATH}"
}

main() {
  require_android_env
  run_android_workflows
  create_consolidated_archive
}

main "$@"
