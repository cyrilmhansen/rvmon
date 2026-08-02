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
  'LET TEXT$="HAMMURABI"' \
  'PRINT MID$(TEXT$,0,1)' \
  'PRINT MID$(TEXT$,1,-1)' \
  'PRINT MID$(TEXT$,1,121)' \
  'PRINT MID$(TEXT$,1)' \
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
    printf 'expected four MID$ diagnostics, got %s\n' "$error_count" >&2
    exit 1
fi
if ! grep -aFq -- 'trap: breakpoint' "$output_file"; then
    cat "$output_file"
    printf 'missing clean stop after MID$ errors\n' >&2
    exit 1
fi
printf 'guest assembly REPL MID$ error QEMU test passed\n'
