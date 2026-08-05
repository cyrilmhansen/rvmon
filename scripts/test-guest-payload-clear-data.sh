#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null
image=target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor
temporary_dir="$(mktemp -d)"
input_fifo="$temporary_dir/input"
output_file="$temporary_dir/output"
mkfifo "$input_fifo"
qemu_pid=""
cleanup() {
    if [[ -n "$qemu_pid" ]] && kill -0 "$qemu_pid" 2>/dev/null; then
        kill "$qemu_pid" 2>/dev/null || true
        wait "$qemu_pid" 2>/dev/null || true
    fi
    rm -rf "$temporary_dir"
}
trap cleanup EXIT

qemu-system-riscv64 -M virt -m 64M -bios none -kernel "$image" \
    -nographic <"$input_fifo" >"$output_file" 2>&1 &
qemu_pid=$!
exec 3>"$input_fifo"

wait_for_text() {
    local text="$1"
    for _ in {1..500}; do
        if grep -aFq -- "$text" "$output_file"; then return 0; fi
        sleep 0.01
    done
    cat "$output_file" >&2
    printf 'timeout waiting for guest text: %s\n' "$text" >&2
    return 1
}

wait_for_text 'rvmonitor> '
printf 'data 0x82000020 .dword 0x1122334455667788\n' >&3
wait_for_text 'stored .dword at 0x0000000082000020 (8 byte(s))'
printf 'payload-clear-data 0x82000000 0x40\n' >&3
wait_for_text 'payload data cleared address=0x0000000082000000 length=0000000000000040'
printf 'memory 0x82000020 8\n' >&3
wait_for_text '0x0000000082000020: 00 00 00 00 00 00 00 00'
printf 'payload-clear-data 0x820ffff0 0x20\n' >&3
wait_for_text 'GUEST-PAYLOAD-011'
printf 'q\n' >&3
exec 3>&-
sleep 0.1
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

printf 'guest payload clear-data QEMU test passed\n'
