#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null
image=target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor

set +e
output="$({
    sleep 0.2
    printf 'basic\n'
    sleep 0.2
    awk '/^### 4.7/{found=1} found && /^```basic/{program=1; next} program && /^```/{exit} program{print}' \
        docs/TUTORIAL-GUEST.md |
        while IFS= read -r line; do
            printf '%s\n' "$line"
            sleep 0.015
        done
    sleep 0.3
    printf 'RUN\n'
    sleep 0.3
    for value in 0 20 190 0 20 190 0 20 190 0 20 190 0 20 190; do
        printf '%s\n' "$value"
        sleep 0.15
    done
    sleep 0.3
    printf 'BYE\nq\n'
} | timeout 15s qemu-system-riscv64 -M virt -m 64M -bios none \
    -kernel "$image" -nographic 2>&1)"
status=$?
set -e

if [[ "$status" -ne 124 ]]; then
    printf '%s\n' "$output"
    printf 'Hammurabi QEMU test exited with status %s, expected timeout 124\n' "$status" >&2
    exit 1
fi

for expected in \
    'HAMMURABI-RV' \
    'FINAL STARVED 0.000000' \
    'GRAIN 1950.000000' \
    'target exit status=0'; do
    if [[ "$output" != *"$expected"* ]]; then
        printf '%s\n' "$output"
        printf 'missing expected output: %s\n' "$expected" >&2
        exit 1
    fi
done

if [[ "$output" == *'ERROR ['* ]]; then
    printf '%s\n' "$output"
    printf 'unexpected error in conservative Hammurabi run\n' >&2
    exit 1
fi

printf 'HAMMURABI-RV target QEMU test passed\n'
