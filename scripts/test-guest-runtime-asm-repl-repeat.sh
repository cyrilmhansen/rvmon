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
sleep 0.3
printf '%s\n' \
  '10 I=0' \
  '20 REPEAT' \
  '30 J=0' \
  '40 REPEAT' \
  '50 J=J+1' \
  '60 PRINT I,J' \
  '70 UNTIL J>=2' \
  '80 I=I+1' \
  '90 UNTIL I>=2' \
  '100 PRINT "DONE"' \
  '110 END' \
  'RUN' >&3
sleep 0.8
printf 'q\n' >&3
exec 3>&-
sleep 0.3
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

for expected in 'DONE' 'trap: breakpoint'; do
    if ! grep -aFq -- "$expected" "$output_file"; then
        cat "$output_file"
        printf 'missing REPEAT/UNTIL output: %s\n' "$expected" >&2
        exit 1
    fi
done
if [[ "$(grep -a -o -- '0\.000000' "$output_file" | wc -l)" -lt 2 ]] ||
   [[ "$(grep -a -o -- '1\.000000' "$output_file" | wc -l)" -lt 2 ]]; then
    cat "$output_file"
    printf 'nested REPEAT/UNTIL did not produce both loop values\n' >&2
    exit 1
fi
printf 'guest assembly REPL REPEAT/UNTIL QEMU test passed\n'
