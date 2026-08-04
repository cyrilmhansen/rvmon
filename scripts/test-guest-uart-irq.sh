#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null
image=target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor

set +e
output="$({
    sleep 0.1
    printf 'assemble-program 0x81000100\n_start:\naddi x1,x0,0\nlui x3,0x10000\nloop:\naddi x1,x1,1\nbne x1,x3,loop\naddi x17,x0,5\necall\naddi x17,x0,3\necall\nend\nrun-at 0x81000100\n'
    # The delay lets run-at reach U-mode before Ctrl-C arrives.  The payload
    # then proves that the UART IRQ was serviced during code that does not
    # call an environment service.
    sleep 0.001
    printf '\003'
    sleep 0.1
    printf 'info uart\nq\n'
} | timeout 5s qemu-system-riscv64 -M virt -m 64M -bios none -kernel "$image" -nographic 2>&1)"
status=$?
set -e

if [[ "$status" -ne 124 ]]; then
    printf '%s\n' "$output"
    printf 'guest UART IRQ test exited with status %s, expected timeout 124\n' "$status" >&2
    exit 1
fi

if [[ "$output" != *'target exit status=3'* || "$output" != *'ctrl-c=1'* ]] ||
    ! [[ "$output" =~ interrupt-services=[1-9][0-9]* ]]; then
    printf '%s\n' "$output"
    printf 'missing independent UART IRQ accounting\n' >&2
    exit 1
fi

printf 'guest UART PLIC interrupt QEMU test passed\n'
