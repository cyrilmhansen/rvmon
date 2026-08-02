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
  '10 PRINT "HAMMURABI-RV"' \
  '20 P=95' \
  '30 A=1000' \
  '40 G=2800' \
  '50 Y=0' \
  '60 T=0' \
  '70 D=0' \
  '80 Y=Y+1' \
  '90 PRINT "YEAR",Y,"PEOPLE",P,"ACRES",A,"GRAIN",G' \
  '100 PRINT "LAND PRICE 10 GRAIN PER ACRE"' \
  '110 PRINT "ACRES TO BUY (NEGATIVE TO SELL)"' \
  '120 INPUT Q' \
  '130 IF Q<0 THEN 180' \
  '140 IF Q*10>G THEN 110' \
  '150 A=A+Q' \
  '160 G=G-Q*10' \
  '170 GOTO 220' \
  '180 Q=0-Q' \
  '190 IF Q>A THEN 110' \
  '200 A=A-Q' \
  '210 G=G+Q*10' \
  '220 PRINT "ACRES TO PLANT"' \
  '230 INPUT Q' \
  '240 IF Q<0 THEN 220' \
  '250 IF Q>A THEN 220' \
  '260 IF Q*2>G THEN 220' \
  '270 G=G-Q*2' \
  '280 H=Q*3' \
  '290 G=G+H' \
  '300 PRINT "BUSHELS TO FEED"' \
  '310 D=0' \
  '320 INPUT Q' \
  '330 IF Q<0 THEN 300' \
  '340 IF Q>G THEN 300' \
  '350 G=G-Q' \
  '360 C=Q/2' \
  '370 IF C>=P THEN 400' \
  '380 D=P-C' \
  '390 P=C' \
  '400 T=T+D' \
  '410 PRINT "HARVEST",H,"STARVED",D' \
  '420 IF D*2>P THEN 500' \
  '430 IF Y<5 THEN 80' \
  '440 GOTO 600' \
  '500 PRINT "REVOLT"' \
  '510 GOTO 600' \
  '600 PRINT "FINAL STARVED",T,"GRAIN",G' \
  '610 END' >&3
sleep 0.4
printf 'RUN\n0\n20\n190\n0\n20\n190\n0\n20\n190\n0\n20\n190\n0\n20\n190\n' >&3
sleep 1.5
printf 'q\n' >&3
exec 3>&-
sleep 0.3
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

for expected in \
    'HAMMURABI-RV' \
    'YEAR' \
    'HARVEST' \
    'FINAL STARVED' \
    '1950.000000' \
    'trap: breakpoint'; do
    if ! grep -aFq -- "$expected" "$output_file"; then
        cat "$output_file"
        printf 'missing integrated Hammurabi output: %s\n' "$expected" >&2
        exit 1
    fi
done
printf 'guest assembly Hammurabi QEMU test passed\n'
