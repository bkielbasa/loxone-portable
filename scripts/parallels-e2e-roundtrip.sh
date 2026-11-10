#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${ROOT_DIR}/target/e2e-roundtrip"
SAMPLE_FILE="${1:-${ROOT_DIR}/Sample.Loxone}"

mkdir -p "${OUT_DIR}"

echo "[1/3] Generating mutated.Loxone and verifying internal round-trip..."
cargo run -p loxone-format-lab -- roundtrip "${SAMPLE_FILE}" --out-dir "${OUT_DIR}"

cat > "${ROOT_DIR}/target/inspect_vm.cmd" << 'EOF'
@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "\\Mac\Home\Projects\loxone-portable\scripts\parallels-roundtrip.ps1"
EOF

rm -f "${OUT_DIR}/vm_status.json"

echo "[2/3] Launching Loxone Config 17 round-trip inside Parallels Windows 11 VM..."
echo "      (Ensure RunCmd.app is invoked via /usr/bin/open -n -a ...)"
