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
    while IFS= read -r line; do
        [[ -z "$line" || "$line" == \;* ]] && continue
        printf '%s\n' "$line" >&3
        sleep 0.01
    done
sleep 0.3
printf '%s\n' \
  '10 PRINT LOG(1)' \
  '20 PRINT LOG(2.718281828459045)' \
  '30 PRINT LOG(10)' \
  '40 PRINT EXP(0)' \
  '50 PRINT EXP(1)' \
  '60 PRINT EXP(-1)' \
  '70 PRINT EXP(LOG(10))' \
  '80 END' \
  'RUN' >&3
sleep 1.0
exec 3>&-
sleep 0.2
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

for expected in \
  '0.000000' \
  '0.999999' \
  '2.302585' \
  '1.000000' \
  '2.718281' \
  '0.367879' \
  '9.999999' \
  'END' \
  'trap: breakpoint'; do
    if ! grep -aFq -- "$expected" "$output_file"; then
        cat "$output_file"
        printf 'missing LOG/EXP result: %s\n' "$expected" >&2
        exit 1
    fi
done
if grep -aFq -- 'ERR' "$output_file"; then
    cat "$output_file"
    printf 'unexpected LOG/EXP diagnostic\n' >&2
    exit 1
fi
printf 'guest assembly REPL LOG/EXP QEMU test passed\n'
