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

assembly_address="$(printf '0x%x' "$((16#$workspace_start_hex + 0x300))")"

set +e
output="$({
    sleep 0.1
    printf 'assemble-program %s\n' "$assembly_address"
    for _ in $(seq 1 8192); do
        printf 'addi x1,x1,0\n'
        sleep 0.001
    done
    printf 'end\nassemble-program 0x%x\n' "$((assembly_address + 0x1000))"
    for _ in $(seq 1 8193); do
        printf 'addi x1,x1,0\n'
        sleep 0.001
    done
    printf 'end\nquit\n'
} | timeout 60s qemu-system-riscv64 \
    -M virt \
    -m 64M \
    -bios none \
    -kernel target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor \
    -nographic 2>&1)"
status=$?
set -e

if [[ "$status" -ne 124 ]]; then
    printf '%s\n' "$output"
    printf 'guest source capacity test exited with status %s, expected timeout 124\n' "$status" >&2
    exit 1
fi

for expected in \
    'assembled program: 8192 instruction(s)' \
    'error [GUEST-ASM-001]: source program exceeds 8192 assembly lines'; do
    if ! [[ "$output" == *"$expected"* ]]; then
        printf '%s\n' "$output"
        printf 'missing expected output: %s\n' "$expected" >&2
        exit 1
    fi
done

printf 'guest source capacity QEMU test passed\n'
