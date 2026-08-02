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
  '10 I=2' \
  '20 ON I GOTO 100,200' \
  '30 PRINT "BAD-DISPATCH"' \
  '40 END' \
  '100 PRINT "BAD-FIRST"' \
  '110 END' \
  '200 I=1' \
  '210 ON I GOSUB 300,400' \
  '220 PRINT "RETURN-OK"' \
  '230 END' \
  '300 PRINT "SUB-OK"' \
  '310 RETURN' \
  '400 PRINT "BAD-SECOND"' \
  '410 END' \
  'RUN' >&3
sleep 0.8
printf 'q\n' >&3
exec 3>&-
sleep 0.3
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

for expected in 'SUB-OK' 'RETURN-OK' 'trap: breakpoint'; do
    if ! grep -aFq -- "$expected" "$output_file"; then
        cat "$output_file"
        printf 'missing ON output: %s\n' "$expected" >&2
        exit 1
    fi
done
for forbidden in 'BAD-'; do
    if grep -aFq -- "$forbidden" "$output_file"; then
        cat "$output_file"
        printf 'unexpected ON result: %s\n' "$forbidden" >&2
        exit 1
    fi
done
printf 'guest assembly REPL ON GOTO/GOSUB QEMU test passed\n'
