#!/usr/bin/env bash
set -euo pipefail

cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null

image=target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor
workspace_start_hex="$(riscv64-linux-gnu-nm -n "$image" \
    | awk '$3 == "_target_workspace_start" && !found { print $1; found=1 }')"
data_start_hex="$(riscv64-linux-gnu-nm -n "$image" \
    | awk '$3 == "_target_data_start" && !found { print $1; found=1 }')"
if [[ -z "$workspace_start_hex" || -z "$data_start_hex" ]]; then
    printf 'could not locate guest linker regions\n' >&2
    exit 1
fi

execution_address="$(printf '0x%x' "$((16#$workspace_start_hex))")"
execution_address_full="$(printf '0x%016x' "$((16#$workspace_start_hex))")"
data_address="$(printf '0x%x' "$((16#$data_start_hex + 0x8))")"
data_address_full="$(printf '0x%016x' "$((16#$data_start_hex + 0x8))")"

set +e
output="$({
    sleep 0.1
    printf 'assemble-program %s\n_start:\nauipc x4,0x1000\naddi x3,x0,42\nsd x3,8(x4)\nld x5,8(x4)\nend\nstep\nregs\nstep\nregs\nstep\nregs\nstep\nregs\nmemory %s 8\nquit\n' \
        "$execution_address" "$data_address"
} | timeout 5s qemu-system-riscv64 \
    -M virt \
    -m 64M \
    -bios none \
    -kernel "$image" \
    -nographic 2>&1)"
status=$?
set -e

if [[ "$status" -ne 124 ]]; then
    printf '%s\n' "$output"
    printf 'guest ld/sd smoke exited with status %s, expected timeout 124\n' "$status" >&2
    exit 1
fi

for expected in \
    "assembled program: 4 instruction(s) at $execution_address_full" \
    'x4=0x0000000082000000' \
    'x5=0x000000000000002a' \
    "$data_address_full: 2a 00 00 00 00 00 00 00"; do
    if ! [[ "$output" == *"$expected"* ]]; then
        printf '%s\n' "$output"
        printf 'missing expected output: %s\n' "$expected" >&2
        exit 1
    fi
done

printf 'guest ld/sd execution smoke test passed\n'
