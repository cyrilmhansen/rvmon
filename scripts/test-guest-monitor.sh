#!/usr/bin/env bash
set -euo pipefail

cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null

set +e
output="$({
    printf 'help\nregs\nstep\nregs\nquit\n'
} | timeout 5s qemu-system-riscv64 \
    -M virt \
    -bios none \
    -kernel target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor \
    -nographic 2>&1)"
status=$?
set -e

if [[ "$status" -ne 124 ]]; then
    printf '%s\n' "$output"
    printf 'guest monitor exited with status %s, expected timeout 124\n' "$status" >&2
    exit 1
fi

for expected in \
    'trap: breakpoint' \
    'help/?  regs/registers  step/s  continue/c  quit/q' \
    'x1=0x0000000000000001' \
    'x1=0x0000000000000002'; do
    if ! [[ "$output" == *"$expected"* ]]; then
        printf '%s\n' "$output"
        printf 'missing expected output: %s\n' "$expected" >&2
        exit 1
    fi
done

printf 'guest monitor QEMU smoke test passed\n'
