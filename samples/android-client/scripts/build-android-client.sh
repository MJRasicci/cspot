#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: build-android-client.sh \
  --project-dir <path> \
  --abi <android-abi> \
  --variant <debug|release> \
  --cspot-lib-dir <path> \
  --cspot-include-dir <path> \
  --output-apk <path>
USAGE
}

log() {
  printf '[android-client-build] %s\n' "$*"
}

die() {
  printf '[android-client-build] ERROR: %s\n' "$*" >&2
  exit 1
}

PROJECT_DIR=""
ABI=""
VARIANT=""
CSPOT_LIB_DIR=""
CSPOT_INCLUDE_DIR=""
OUTPUT_APK=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --project-dir)
      PROJECT_DIR="$2"
      shift 2
      ;;
    --abi)
      ABI="$2"
      shift 2
      ;;
    --variant)
      VARIANT="$2"
      shift 2
      ;;
    --cspot-lib-dir)
      CSPOT_LIB_DIR="$2"
      shift 2
      ;;
    --cspot-include-dir)
      CSPOT_INCLUDE_DIR="$2"
      shift 2
      ;;
    --output-apk)
      OUTPUT_APK="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

[ -n "${PROJECT_DIR}" ] || die "--project-dir is required"
[ -n "${ABI}" ] || die "--abi is required"
[ -n "${VARIANT}" ] || die "--variant is required"
[ -n "${CSPOT_LIB_DIR}" ] || die "--cspot-lib-dir is required"
[ -n "${CSPOT_INCLUDE_DIR}" ] || die "--cspot-include-dir is required"
[ -n "${OUTPUT_APK}" ] || die "--output-apk is required"

case "${VARIANT}" in
  debug|release)
    ;;
  *)
    die "--variant must be debug or release"
    ;;
esac

[ -d "${PROJECT_DIR}" ] || die "project directory not found: ${PROJECT_DIR}"
[ -f "${CSPOT_LIB_DIR}/libcspot.so" ] || die "missing libcspot.so in ${CSPOT_LIB_DIR}"
[ -f "${CSPOT_INCLUDE_DIR}/cspot.h" ] || die "missing cspot.h in ${CSPOT_INCLUDE_DIR}"

if [ -z "${ANDROID_SDK_ROOT:-}" ] && [ -z "${ANDROID_HOME:-}" ]; then
  die "ANDROID_SDK_ROOT or ANDROID_HOME must be set"
fi

REPO_ROOT="$(cd "${PROJECT_DIR}/../.." && pwd)"
TOOLS_DIR="${REPO_ROOT}/artifacts/tools"
GRADLE_VERSION="${CSPOT_ANDROID_GRADLE_VERSION:-8.7}"
GRADLE_DIR="${TOOLS_DIR}/gradle-${GRADLE_VERSION}"
GRADLE_BIN="${GRADLE_DIR}/bin/gradle"
GRADLE_HOME="${REPO_ROOT}/artifacts/gradle-home"

mkdir -p "${TOOLS_DIR}" "${GRADLE_HOME}"

if [ ! -x "${GRADLE_BIN}" ]; then
  ARCHIVE="${TOOLS_DIR}/gradle-${GRADLE_VERSION}-bin.zip"
  log "Downloading Gradle ${GRADLE_VERSION}"
  curl -fsSL "https://services.gradle.org/distributions/gradle-${GRADLE_VERSION}-bin.zip" -o "${ARCHIVE}"
  rm -rf "${GRADLE_DIR}"
  unzip -q "${ARCHIVE}" -d "${TOOLS_DIR}"
fi

case "${VARIANT}" in
  debug)
    TASK="assembleDebug"
    ;;
  release)
    TASK="assembleRelease"
    ;;
  *)
    die "--variant must be debug or release"
    ;;
esac

log "Building android-client (${ABI} ${VARIANT})"
GRADLE_USER_HOME="${GRADLE_HOME}" "${GRADLE_BIN}" \
  --no-daemon \
  -p "${PROJECT_DIR}" \
  "-PcspotAbi=${ABI}" \
  "-PcspotLibDir=${CSPOT_LIB_DIR}" \
  "-PcspotIncludeDir=${CSPOT_INCLUDE_DIR}" \
  ":app:${TASK}"

APK_OUTPUT_DIR="${REPO_ROOT}/artifacts/android-client/gradle/${ABI}/app/build/outputs/apk/${VARIANT}"
APK_METADATA="${APK_OUTPUT_DIR}/output-metadata.json"
APK_SOURCE=""

if [ -f "${APK_METADATA}" ]; then
  APK_FILE_NAME="$(sed -n 's/.*"outputFile"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "${APK_METADATA}" | head -n 1)"
  if [ -n "${APK_FILE_NAME}" ]; then
    APK_SOURCE="${APK_OUTPUT_DIR}/${APK_FILE_NAME}"
  fi
fi

if [ -z "${APK_SOURCE}" ] || [ ! -f "${APK_SOURCE}" ]; then
  for candidate in "app-${VARIANT}.apk" "app-${VARIANT}-unsigned.apk"; do
    if [ -f "${APK_OUTPUT_DIR}/${candidate}" ]; then
      APK_SOURCE="${APK_OUTPUT_DIR}/${candidate}"
      break
    fi
  done
fi

[ -n "${APK_SOURCE}" ] && [ -f "${APK_SOURCE}" ] || die "expected APK missing in ${APK_OUTPUT_DIR}"
log "Using APK ${APK_SOURCE}"

mkdir -p "$(dirname "${OUTPUT_APK}")"
cp "${APK_SOURCE}" "${OUTPUT_APK}"
log "Wrote ${OUTPUT_APK}"
