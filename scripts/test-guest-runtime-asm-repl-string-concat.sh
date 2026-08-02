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
  'LET LONGTEXT$="NILE"' \
  'LET COPIED$=LONGTEXT$' \
  'LET LONGTEXT2$=LONGTEXT$+" RIVER"' \
  'LET PREFIX$="RV "+TEXT$' \
  'LET COMPOSED$=PREFIX$+"!"' \
  'LET WITHLEFT$=">"+LEFT$(TEXT$,4)' \
  'LET WITHRIGHT$=RIGHT$(TEXT$,4)+"<"' \
  'LET WITHMID$="["+MID$(TEXT$,2,4)+"]"' \
  'DIM A$(1)' \
  'LET A$(0)=TEXT$+" GAME"' \
  '10 PRINT PREFIX$' \
  '20 PRINT COMPOSED$' \
  '30 PRINT A$(0)' \
  '40 PRINT WITHLEFT$' \
  '50 PRINT WITHRIGHT$' \
  '60 PRINT WITHMID$' \
  '70 PRINT COPIED$' \
  '80 PRINT LONGTEXT2$' \
  '90 END' \
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

for expected in 'RV HAMMURABI' 'RV HAMMURABI!' 'HAMMURABI GAME' '>HAMM<' 'RABI<' '[AMMU]' 'NILE' 'NILE RIVER' 'trap: breakpoint'; do
    if ! grep -aFq -- "$expected" "$output_file"; then
        cat "$output_file"
        printf 'missing string-concat result: %s\n' "$expected" >&2
        exit 1
    fi
done
if grep -aFq -- 'ERR' "$output_file"; then
    cat "$output_file"
    printf 'unexpected string-concat diagnostic\n' >&2
    exit 1
fi
printf 'guest assembly REPL string concatenation QEMU test passed\n'
