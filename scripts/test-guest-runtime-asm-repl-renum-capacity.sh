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

long_line='10 GOTO 20'
for _ in $(seq 1 12); do
    long_line+=":GOTO 20"
done
printf '%s\n' \
  "$long_line" \
  '20 END' \
  'RENUM 1000,10,10' \
  'LIST' \
  'q' | while IFS= read -r line; do
    printf '%s\n' "$line" >&3
    sleep 0.15
done
sleep 0.8
exec 3>&-
sleep 0.2
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

if ! grep -aFq -- 'ERR' "$output_file"; then
    cat "$output_file"
    printf 'missing RENUM capacity diagnostic\n' >&2
    exit 1
fi
if ! grep -aFq -- "$long_line" "$output_file"; then
    cat "$output_file"
    printf 'RENUM capacity rejection changed the line body\n' >&2
    exit 1
fi
if ! grep -aFq -- '20 END' "$output_file" || grep -aFq -- '1000 GOTO' "$output_file"; then
    cat "$output_file"
    printf 'RENUM capacity rejection changed line numbers\n' >&2
    exit 1
fi
if grep -aFq -- 'mcause=' "$output_file"; then
    cat "$output_file"
    printf 'RENUM capacity rejection caused a target fault\n' >&2
    exit 1
fi
printf 'guest assembly REPL RENUM capacity QEMU test passed\n'
