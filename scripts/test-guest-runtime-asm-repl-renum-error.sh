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
printf 'basic\n' >&3
sleep 0.3
printf '%s\n' \
  '10 PRINT "A"' \
  '30 END' \
  'RENUM 100,10,0' \
  'LIST' \
  'q' >&3
sleep 0.5
exec 3>&-
sleep 0.2
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

if ! grep -aFq -- 'ERR' "$output_file"; then
    cat "$output_file"
    printf 'missing RENUM reference diagnostic\n' >&2
    exit 1
fi
if ! grep -aFq -- '10 PRINT "A"' "$output_file"; then
    cat "$output_file"
    printf 'RENUM error changed the program unexpectedly\n' >&2
    exit 1
fi
if grep -aFq -- '100 PRINT "A"' "$output_file"; then
    cat "$output_file"
    printf 'RENUM wrote a partial result after rejecting references\n' >&2
    exit 1
fi
printf 'guest assembly REPL RENUM validation-error QEMU test passed\n'
