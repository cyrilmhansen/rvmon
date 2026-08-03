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
sleep 0.1
awk '/^symbols$/{print; found=1; next} found && /^run-at /{print; exit} !found{print}' \
    examples/minibasic-asm/payload-repl.rv |
    while IFS= read -r line; do printf '%s\n' "$line" >&3; sleep 0.003; done
sleep 1.0
printf '%s\n' \
  '10 PRINT 2+3*4' \
  '20 END' \
  'RUN' \
  'memory 0x820622f0 8' \
  'memory 0x82062300 24' \
  'q' >&3
exec 3>&-
sleep 0.4
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

for expected in \
  '14.000000' \
  '0x00000000820622f0: 05 00 00 00 00 00 00 00' \
  '0x0000000082062300: 01 00 00 00 00 00 00 00'; do
    if [[ "$(<"$output_file")" != *"$expected"* ]]; then
        cat "$output_file"
        printf 'missing expression token evidence: %s\n' "$expected" >&2
        exit 1
    fi
done
printf 'guest expression lexer token QEMU test passed\n'
