#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null
image=target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor

set +e
output="$({
    sleep 0.1
    printf 'info uart\n'
    sleep 0.1
    printf 'q\n'
} | timeout 5s qemu-system-riscv64 -M virt -m 64M -bios none -kernel "$image" -nographic 2>&1)"
status=$?
set -e

if [[ "$status" -ne 124 ]]; then
    printf '%s\n' "$output"
    printf 'guest UART test exited with status %s, expected timeout 124\n' "$status" >&2
    exit 1
fi

for expected in \
    'uart 16550A base=0x0000000010000000 mode=interrupt+polling-fallback' \
    'rx-queued=0 interrupt-services=0 hardware-overruns=0 software-drops=0' \
    'parity-errors=0 framing-errors=0 breaks=0 ctrl-c=0'; do
    if [[ "$output" != *"$expected"* ]]; then
        printf '%s\n' "$output"
        printf 'missing expected UART output: %s\n' "$expected" >&2
        exit 1
    fi
done

printf 'guest UART 16550A driver QEMU test passed\n'
