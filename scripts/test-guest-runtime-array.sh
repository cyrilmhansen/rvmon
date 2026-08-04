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
    -nographic < "$input_fifo" > "$output_file" 2>&1 &
qemu_pid=$!
exec 3>"$input_fifo"
sleep 0.1
cat examples/minibasic-runtime-array.rv >&3
exec 3>&-
sleep 0.3
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""
output="$(<"$output_file")"

for expected in \
    'assembled program:' \
    'target exit status=0' \
    '0x0000000082000200: 00 04 00 82 00 00 00 00 0b 00 00 00 00 00 00 00' \
    '0x0000000082000210: 08 00 00 00 01 00 00 00 01 00 00 00 00 00 00 00' \
    '0x0000000082000220: 0a 00 00 00 00 00 00 00' \
    '0x0000000082000400: 00 00 00 00 00 00 45 40'; do
    if [[ "$output" != *"$expected"* ]]; then
        printf '%s\n' "$output"
        printf 'missing expected runtime array output: %s\n' "$expected" >&2
        exit 1
    fi
done

printf 'guest runtime array QEMU test passed\n'
