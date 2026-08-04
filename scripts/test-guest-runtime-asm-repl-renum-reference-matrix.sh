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
    while IFS= read -r line; do
        [[ -z "$line" || "$line" == \;* ]] && continue
        printf '%s\n' "$line" >&3
        sleep 0.003
    done
sleep 0.2
printf '%s\n' \
  '10 X=1' \
  '20 GOSUB 80' \
  '30 ON 1 GOSUB 100,110' \
  '40 IF X=1 THEN 120' \
  '50 PRINT "BAD-FLOW"' \
  '60 END' \
  '80 PRINT "SUB"' \
  '90 RETURN' \
  '100 PRINT "ON-SUB"' \
  '110 RETURN' \
  '120 PRINT "THEN"' \
  '130 PRINT "literal 20 30"' \
  '140 END' \
  'RENUM 1000,20,10' \
  'LIST' \
  'RUN' \
  'q' | while IFS= read -r line; do
    printf '%s\n' "$line" >&3
    sleep 0.08
done
sleep 0.8
exec 3>&-
sleep 0.2
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

for expected in \
    '10 X=1' \
    '20 GOSUB 80' \
    '1000 GOSUB 1050' \
    '1010 ON 1 GOSUB 1070,1080' \
    '1020 IF X=1 THEN 1090' \
    '1080 RETURN' \
    '1100 PRINT "literal 20 30"' \
    'SUB' \
    'ON-SUB' \
    'THEN' \
    'literal 20 30' \
    'trap: breakpoint'; do
    if ! grep -aFq -- "$expected" "$output_file"; then
        cat "$output_file"
        printf 'missing RENUM reference-matrix output: %s\n' "$expected" >&2
        exit 1
    fi
done
for forbidden in 'BAD-FLOW' 'BAD-ON' 'ERR'; do
    if grep -aFxq -- "$forbidden" "$output_file"; then
        cat "$output_file"
        printf 'unexpected RENUM reference-matrix output: %s\n' "$forbidden" >&2
        exit 1
    fi
done
printf 'guest assembly REPL RENUM reference-matrix QEMU test passed\n'
