#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null
image=target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor
workspace_start_hex="$(riscv64-linux-gnu-nm -n "$image" |
    awk '$3 == "_target_workspace_start" && !found { print $1; found=1 }')"
address="$(printf '0x%x' "$((16#$workspace_start_hex + 0x140))")"

set +e
output="$({
    sleep 0.1
    printf 'assemble-program %s\nfdiv.d f3,f1,f2\nebreak\nend\n' "$address"
    printf 'setf f1 0x3ff0000000000000\nsetf f2 0x4000000000000000\nstep\nregs\nquit\n'
} | timeout 5s qemu-system-riscv64 -M virt -m 64M -bios none -kernel "$image" -nographic 2>&1)"
status=$?
set -e

if [[ "$status" -ne 124 ]]; then
    printf '%s\n' "$output"
    printf 'guest fdiv test exited with status %s, expected timeout 124\n' "$status" >&2
    exit 1
fi
for expected in \
    'assembled program: 2 instruction(s)' \
    'f3=0x3fe0000000000000' \
    'fcsr=0x00000000'; do
    if [[ "$output" != *"$expected"* ]]; then
        printf '%s\n' "$output"
        printf 'missing expected output: %s\n' "$expected" >&2
        exit 1
    fi
done
printf 'guest fdiv.d execution QEMU test passed\n'
