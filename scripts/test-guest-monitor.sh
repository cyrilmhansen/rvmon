#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null
image=target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor
workspace_hex="$(riscv64-linux-gnu-nm -n "$image" |
    awk '$3 == "_target_workspace_start" && !found { print $1; found=1 }')"
data_hex="$(riscv64-linux-gnu-nm -n "$image" |
    awk '$3 == "_target_data_start" && !found { print $1; found=1 }')"
assembly_address="$(printf '0x%x' "$((16#$workspace_hex + 0x100))")"
data_address="$(printf '0x%x' "$((16#$data_hex + 0x60))")"

set +e
output="$({
    sleep 0.1
    printf 'help\ninfo uart\nregs\n'
    printf 'set x9 0x8000000080000000\nset x0 0x1\n'
    printf 'assemble %s addi x1,x0,1\ndisasm %s 1\n' "$assembly_address" "$assembly_address"
    printf 'data %s .word 0x11223344\nmemory %s 4\nq\n' "$data_address" "$data_address"
} | timeout 5s qemu-system-riscv64 -M virt -m 64M -bios none -kernel "$image" -nographic 2>&1)"
status=$?
set -e

if [[ "$status" -ne 124 ]]; then
    printf '%s\n' "$output"
    printf 'guest monitor smoke test exited with status %s, expected timeout 124\n' "$status" >&2
    exit 1
fi

for expected in \
    'RVMonitor 4B M-mode' \
    'help' \
    'mode=interrupt+polling-fallback' \
    'integer registers:' \
    'set x9=0x8000000080000000' \
    'error: x0 is read-only' \
    'assembled instruction at' \
    'addi x1,x0,1' \
    'stored .word at' \
    '44 33 22 11'; do
    if [[ "$output" != *"$expected"* ]]; then
        printf '%s\n' "$output"
        printf 'missing expected output: %s\n' "$expected" >&2
        exit 1
    fi
done

printf 'guest monitor smoke QEMU test passed\n'
