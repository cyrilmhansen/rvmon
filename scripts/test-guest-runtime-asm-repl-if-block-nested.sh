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
sleep 0.3
printf '%s\n' \
  '10 A=1' \
  '20 IF A=1 THEN' \
  '30 B=0' \
  '40 IF B=1 THEN' \
  '50 PRINT "BAD-INNER-THEN"' \
  '60 ELSE' \
  '70 PRINT "INNER-ELSE-OK"' \
  '80 ENDIF' \
  '90 ELSE' \
  '100 PRINT "BAD-OUTER-ELSE"' \
  '110 ENDIF' \
  '120 A=0' \
  '130 IF A=1 THEN' \
  '140 IF B=1 THEN' \
  '150 PRINT "BAD-NESTED-THEN"' \
  '160 ELSE' \
  '170 PRINT "BAD-NESTED-ELSE"' \
  '180 ENDIF' \
  '190 ELSE' \
  '200 PRINT "OUTER-ELSE-OK"' \
  '210 ENDIF' \
  '220 END' \
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

for expected in 'INNER-ELSE-OK' 'OUTER-ELSE-OK' 'trap: breakpoint'; do
    if ! grep -aFq -- "$expected" "$output_file"; then
        cat "$output_file"
        printf 'missing nested IF block output: %s\n' "$expected" >&2
        exit 1
    fi
done
for forbidden in 'BAD-' 'ERR'; do
    if grep -aFq -- "$forbidden" "$output_file"; then
        cat "$output_file"
        printf 'unexpected nested IF block result: %s\n' "$forbidden" >&2
        exit 1
    fi
done
printf 'guest assembly REPL nested IF/ELSE/ENDIF QEMU test passed\n'
