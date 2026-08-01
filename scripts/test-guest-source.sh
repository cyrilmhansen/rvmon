#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null

workspace_start_hex="$(riscv64-linux-gnu-nm -n target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor |
    awk '$3 == "_target_workspace_start" && !found { print $1; found=1 }')"
if [[ -z "$workspace_start_hex" ]]; then
    printf 'could not locate guest workspace symbol\n' >&2
    exit 1
fi

assembly_address="$(printf '0x%x' "$((16#$workspace_start_hex + 0x1c0))")"
assembly_address_full="$(printf '0x%016x' "$((16#$workspace_start_hex + 0x1c0))")"

set +e
output="$({
    sleep 0.1
    printf 'assemble-program %s\naddi x1,x0,1\naddi x1,x1,2\nend\n' "$assembly_address"
    printf 'source\nsource 2\nsource replace 2 "addi x1,x1,5"\nsource 2\ndisasm %s 2\nrun 0\nassemble-source\nrun 2\nregs\n' "$assembly_address"
    printf 'source 2\nquit\n'
} | timeout 5s qemu-system-riscv64 \
    -M virt \
    -m 64M \
    -bios none \
    -kernel target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor \
    -nographic 2>&1)"
status=$?
set -e

if [[ "$status" -ne 124 ]]; then
    printf '%s\n' "$output"
    printf 'guest source test exited with status %s, expected timeout 124\n' "$status" >&2
    exit 1
fi

for expected in \
    '1 | addi x1,x0,1' \
    '2 | addi x1,x1,2' \
    'source line 2 updated; use assemble-source to apply' \
    '2 | addi x1,x1,5' \
    'error [GUEST-RUN-003]' \
    "assembled source: 2 instruction(s) at $assembly_address_full" \
    'run: budget exhausted' \
    'x1=0x0000000000000006'; do
    if ! [[ "$output" == *"$expected"* ]]; then
        printf '%s\n' "$output"
        printf 'missing expected output: %s\n' "$expected" >&2
        exit 1
    fi
done

printf 'guest source replacement QEMU test passed\n'
