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
sleep 0.3
printf '%s\n' \
  'DIM A$(2)' \
  'A$(0)="HAMMURABI"' \
  'LET A$(1)=RIGHT$(A$(0),-1)' \
  'LET A$(1)=MID$(A$(0),0,2)' \
  'LET A$(1)=LEFT$(A$(0),121)' \
  'LET A$(1)=MID$(A$(0))' \
  '10 END' \
  'RUN' | while IFS= read -r line; do
    printf '%s\n' "$line" >&3
    sleep 0.08
  done
sleep 1.0
printf 'q\n' >&3
exec 3>&-
sleep 0.2
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

error_count=$(grep -aF -- 'ERR' "$output_file" | wc -l)
if (( error_count < 4 )); then
    cat "$output_file"
    printf 'expected four array-destination slice diagnostics, got %s\n' "$error_count" >&2
    exit 1
fi
if ! grep -aFq -- 'trap: breakpoint' "$output_file"; then
    cat "$output_file"
    printf 'missing clean stop after array-destination slice errors\n' >&2
    exit 1
fi
printf 'guest assembly REPL string slice array-destination error QEMU test passed\n'
