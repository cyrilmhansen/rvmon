#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null
image=target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor
workspace_start_hex="$(riscv64-linux-gnu-nm -n "$image" |
    awk '$3 == "_target_workspace_start" && !found { print $1; found=1 }')"
address="$(printf '0x%x' "$((16#$workspace_start_hex + 0x100))")"

set +e
output="$({
    sleep 0.1
    printf 'info payload\n'
    printf 'assemble-program %s\n' "$address"
    printf 'addi x10,x0,65\naddi x17,x0,1\necall\naddi x10,x0,0\naddi x17,x0,3\necall\nend\n'
    printf 'run-at %s\n' "$address"
    printf 'q\n'
} | timeout 5s qemu-system-riscv64 -M virt -m 64M -bios none -kernel "$image" -nographic 2>&1)"
status=$?
set -e

if [[ "$status" -ne 124 ]]; then
    printf '%s\n' "$output"
    printf 'run-at QEMU test exited with status %s, expected timeout 124\n' "$status" >&2
    exit 1
fi
for expected in \
    'payload abi=RVMPAY01 profile=RV64ILP32D-MON-1 endian=little' \
    'workspace=0x0000000081000000..0x0000000081010000 bytes=65536' \
    'data=0x0000000082000000..0x0000000082100000 bytes=1048576' \
    'entry-alignment=4 u-stack-bytes=8192 m-stack-bytes=65536' \
    'assembled program: 6 instruction(s)' \
    'A' \
    'target exit status=0'; do
    if [[ "$output" != *"$expected"* ]]; then
        printf '%s\n' "$output"
        printf 'missing expected output: %s\n' "$expected" >&2
        exit 1
    fi
done
printf 'guest run-at payload QEMU test passed\n'
