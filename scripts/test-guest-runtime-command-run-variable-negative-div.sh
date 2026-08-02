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
sed '/^regs$/,$d' examples/minibasic-runtime-command-run-variable-negative-div.rv >&3
printf 'RUN\n' >&3
sleep 0.2
printf 'regs\nmemory 0x82000400 8\nq\n' >&3
exec 3>&-
sleep 0.3
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

for expected in \
    'trap: breakpoint' \
    'f3=0xc004000000000000' \
    '0x0000000082000400: 00 00 00 00 00 00 04 c0'; do
    if ! grep -aFq "$expected" "$output_file"; then
        cat "$output_file"
        printf 'missing target negative division output: %s\n' "$expected" >&2
        exit 1
    fi
done

printf 'guest RUN negative variable division QEMU test passed\n'
