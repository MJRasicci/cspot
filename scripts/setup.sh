#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

case "$(uname -s)" in
  Darwin)
    exec "${SCRIPT_DIR}/setup/setup-macos.sh" "$@"
    ;;
  Linux)
    exec "${SCRIPT_DIR}/setup/setup-linux.sh" "$@"
    ;;
  *)
    echo "Unsupported OS. Use setup-windows.ps1 on Windows." >&2
    exit 1
    ;;
esac
