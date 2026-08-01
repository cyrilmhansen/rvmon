#!/usr/bin/env bash
set -uo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

strict=0
if [[ "${1:-}" == "--strict" ]]; then strict=1; elif [[ $# -gt 0 ]]; then
    printf 'usage: tools/e2e-release-smoke.sh [--strict]\n' >&2
    exit 2
fi

scripts=(
    scripts/test-guest-monitor.sh
    scripts/test-guest-ecall.sh
    scripts/test-guest-run-at.sh
    scripts/test-guest-payload-skeleton.sh
    scripts/test-guest-fdiv.sh
    scripts/test-minibasic.sh
    scripts/test-hammurabi.sh
    scripts/test-guest-run.sh
    scripts/test-guest-ld-sd.sh
    scripts/test-guest-source.sh
    scripts/test-guest-watchpoint.sh
    scripts/test-guest-snapshot.sh
    scripts/test-guest-snapshot-binary.sh
    scripts/test-guest-snapshot-export.sh
    scripts/test-qemu-gdb-backend.sh
)

printf 'RVMonitor E2E release smoke\nprofile=guest-qemu-mmode-umode\nstrict=%s\n\n' "$strict"
failures=0
skipped=0
for script in "${scripts[@]}"; do
    printf '[E2E] %s\n' "$script"
    if [[ ! -x "$script" ]]; then
        printf '[SKIP] not executable\n\n'
        skipped=$((skipped + 1))
        continue
    fi
    if ! command -v qemu-system-riscv64 >/dev/null 2>&1; then
        printf '[SKIP] qemu-system-riscv64 unavailable\n\n'
        skipped=$((skipped + 1))
        continue
    fi
    if bash "$script"; then
        printf '[PASS] %s\n\n' "$script"
    else
        printf '[FAIL] %s\n\n' "$script"
        failures=$((failures + 1))
    fi
done

printf 'SUMMARY=failures:%s skipped:%s total:%s\n' "$failures" "$skipped" "${#scripts[@]}"
if ((failures > 0 || (strict && skipped > 0))); then exit 1; fi
