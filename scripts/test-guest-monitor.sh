#!/usr/bin/env bash
set -euo pipefail

cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null

target_entry_hex="$(riscv64-linux-gnu-nm -n target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor \
    | awk '$3 == "target_entry" && !found { print $1; found=1 }')"
workspace_start_hex="$(riscv64-linux-gnu-nm -n target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor \
    | awk '$3 == "_target_workspace_start" && !found { print $1; found=1 }')"
if [[ -z "$target_entry_hex" ]]; then
    printf 'could not locate target_entry in guest image\n' >&2
    exit 1
fi
if [[ -z "$workspace_start_hex" ]]; then
    printf 'could not locate _target_workspace_start in guest image\n' >&2
    exit 1
fi
breakpoint_address="$(printf '0x%x' "$((16#$target_entry_hex + 12))")"
assembly_address="$(printf '0x%x' "$((16#$workspace_start_hex + 0x100))")"
assembly_address_full="$(printf '0x%016x' "$((16#$workspace_start_hex + 0x100))")"

set +e
output="$({
    printf 'help\nregs\nmemory 0x80000000 16\nbreak %s\ninfo break\ncontinue\nregs\ncontinue\ndelete 1\nstep\nregs\nstep\nregs\nassemble-program %s\n_start:\naddi x1,x0,1\nbeq x1,x1,next\naddi x1,x0,99\nnext:\naddi x1,x1,2\nend\nsymbols\ndisasm _start 4\nbreak next\ninfo break\ndelete 1\nstep\nregs\nstep\nregs\nstep\nregs\nquit\n' \
        "$breakpoint_address" "$assembly_address"
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
    'breakpoint #1 set at' \
    'breakpoints:' \
    'breakpoint #1 deleted' \
    'integer registers:' \
    'floating registers (raw bits):' \
    '0x0000000080000000:' \
    'x31=0x' \
    'f31=0x' \
    'source mode: enter addi, beq, bne, jal or jalr lines, finish with end' \
    "assembled program: 4 instruction(s) at $assembly_address_full" \
    '_start' \
    'next' \
    'addi x1,x0,1' \
    'beq x1,x1,next' \
    'addi x1,x0,99' \
    'addi x1,x1,2' \
    'breakpoint #1 set at' \
    'breakpoint #1 deleted' \
    'x1=0x0000000000000003' \
    'x1=0x0000000000000001'; do
    if ! [[ "$output" == *"$expected"* ]]; then
        printf '%s\n' "$output"
        printf 'missing expected output: %s\n' "$expected" >&2
        exit 1
    fi
done

printf 'guest monitor QEMU smoke test passed\n'
