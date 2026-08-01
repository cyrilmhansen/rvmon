#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null
image=target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor
workspace_start_hex="$(riscv64-linux-gnu-nm -n "$image" |
    awk '$3 == "_target_workspace_start" && !found { print $1; found=1 }')"
[[ -n "$workspace_start_hex" ]] || { printf 'workspace symbol missing\n' >&2; exit 1; }
address="$(printf '0x%x' "$((16#$workspace_start_hex + 0x100))")"

set +e
output="$({
    sleep 0.1
    printf 'assemble-program %s\n' "$address"
    printf 'addi x10,x0,65\naddi x17,x0,1\necall\naddi x17,x0,2\necall\naddi x17,x0,1\necall\naddi x10,x0,0\naddi x17,x0,3\necall\nend\n'
    printf 'continue\n'
    printf 'q\n'
} | timeout 5s qemu-system-riscv64 -M virt -m 64M -bios none -kernel "$image" -nographic 2>&1)"
status=$?
set -e

if [[ "$status" -ne 124 ]]; then
    printf '%s\n' "$output"
    printf 'guest ecall test exited with status %s, expected timeout 124\n' "$status" >&2
    exit 1
fi
for expected in \
    'assembled program: 10 instruction(s)' \
    'A' \
    'target exit status=0' \
    'rvmonitor> '; do
    if [[ "$output" != *"$expected"* ]]; then
        printf '%s\n' "$output"
        printf 'missing expected output: %s\n' "$expected" >&2
        exit 1
    fi
done
printf 'guest ecall service ABI QEMU test passed\n'
