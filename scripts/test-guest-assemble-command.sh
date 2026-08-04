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
sleep 0.2
printf '%s\n' \
  'assemble 0x81000100 addi x1,x0,1' \
  'regs' \
  'q' >&3
sleep 0.8
exec 3>&-
sleep 0.2
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

for expected in \
    'assembled instruction at 0x0000000081000100 = 0x0000000000100093' \
    'pc=0x0000000081000100 mepc=0x0000000081000100'; do
    if ! grep -aFq -- "$expected" "$output_file"; then
        cat "$output_file"
        printf 'missing assemble-command output: %s\n' "$expected" >&2
        exit 1
    fi
done
if grep -aFq -- 'mcause=0x0000000000000002' "$output_file"; then
    cat "$output_file"
    printf 'single-instruction assemble caused an illegal-instruction trap\n' >&2
    exit 1
fi
printf 'guest assemble command QEMU regression passed\n'
