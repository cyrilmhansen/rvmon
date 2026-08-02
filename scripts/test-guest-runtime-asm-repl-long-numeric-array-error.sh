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
    while IFS= read -r line; do printf '%s\n' "$line" >&3; sleep 0.002; done
sleep 0.2
printf 'DIM LONGNUM(2)\nLONGNUM(3)=9\nLONGNUM(1)=7\n10 PRINT LONGNUM(1)\n20 END\nRUN\n' >&3
sleep 1.0
printf 'q\n' >&3
exec 3>&-
sleep 0.3
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

for expected in 'ERR' '7.000000' 'trap: breakpoint'; do
    if ! grep -aFq -- "$expected" "$output_file"; then
        cat "$output_file"
        printf 'missing long numeric-array error output: %s\n' "$expected" >&2
        exit 1
    fi
done
printf 'guest assembly REPL long-numeric-array error QEMU test passed\n'
