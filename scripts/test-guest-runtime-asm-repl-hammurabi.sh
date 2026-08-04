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

wait_for_text() {
    local text="$1"
    for _ in {1..2000}; do
        if grep -aFq -- "$text" "$output_file"; then return 0; fi
        sleep 0.01
    done
    cat "$output_file" >&2
    printf 'timeout waiting for guest text: %s\n' "$text" >&2
    return 1
}

sleep 0.1
printf 'basic\n' >&3
wait_for_text 'READY> '
printf '%s\n' \
  '10 PRINT "HAMMURABI-RV":PRINT "GOVERN SUMER":PRINT "ANCIENT SUMER":PRINT "FIVE YEAR TERM"' \
  '20 CITIZENS=95' \
  '30 HOLDINGS=1000' \
  '40 CORNSTOCK=2800' \
  '50 REGNALYEAR=0' \
  '60 OVERALLDEATH=0' \
  '70 MORTALITY=0' \
  '80 REGNALYEAR=REGNALYEAR+1' \
  '90 PRINT "YEAR",REGNALYEAR,"PEOPLE",CITIZENS,"ACRES",HOLDINGS,"GRAIN",CORNSTOCK' \
  '100 PRINT "LAND PRICE 10 GRAIN PER ACRE"' \
  '110 PRINT "ACRES TO BUY (NEGATIVE TO SELL)"' \
  '120 INPUT QUANTITY' \
  '130 IF QUANTITY<0 THEN 180' \
  '140 IF QUANTITY*10>CORNSTOCK THEN 110' \
  '150 HOLDINGS=HOLDINGS+QUANTITY' \
  '160 CORNSTOCK=CORNSTOCK-QUANTITY*10' \
  '170 GOTO 220' \
  '180 QUANTITY=0-QUANTITY' \
  '190 IF QUANTITY>HOLDINGS THEN 110' \
  '200 HOLDINGS=HOLDINGS-QUANTITY' \
  '210 CORNSTOCK=CORNSTOCK+QUANTITY*10' \
  '220 PRINT "ACRES TO PLANT"' \
  '230 INPUT QUANTITY' \
  '240 IF QUANTITY<0 THEN 220' \
  '250 IF QUANTITY>HOLDINGS THEN 220' \
  '260 IF QUANTITY*2>CORNSTOCK THEN 220' \
  '270 CORNSTOCK=CORNSTOCK-QUANTITY*2' \
  '280 HARVESTED=QUANTITY*3' \
  '290 CORNSTOCK=CORNSTOCK+HARVESTED' \
  '300 PRINT "BUSHELS TO FEED"' \
  '310 MORTALITY=0' \
  '320 INPUT QUANTITY' \
  '330 IF QUANTITY<0 THEN 300' \
  '340 IF QUANTITY>CORNSTOCK THEN 300' \
  '350 CORNSTOCK=CORNSTOCK-QUANTITY' \
  '360 CITIZENFED=QUANTITY/2' \
  '370 IF CITIZENFED>=CITIZENS THEN 400' \
  '380 MORTALITY=CITIZENS-CITIZENFED' \
  '390 CITIZENS=CITIZENFED' \
  '400 OVERALLDEATH=OVERALLDEATH+MORTALITY' \
  '410 PRINT "HARVEST",HARVESTED,"STARVED",MORTALITY' \
  '420 IF MORTALITY*2>CITIZENS THEN 500' \
  '430 IF REGNALYEAR<5 THEN 80' \
  '440 GOTO 600' \
  '500 PRINT "REVOLT"' \
  '510 GOTO 600' \
  '600 PRINT "FINAL STARVED",OVERALLDEATH,"GRAIN",CORNSTOCK' \
  '610 END' >&3
sleep 0.4
printf 'DUMP\nTRACE ON\nRUN\n' >&3
for value in 0 20 190 0 20 190 0 20 190 0 20 190 0 20 190; do
    printf '%s\n' "$value" >&3
    sleep 0.15
done
sleep 1
exec 3>&-
sleep 0.3
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

for expected in \
    'HAMMURABI-RV' \
    'DUMP' \
    'GOVERN SUMER' \
    'ANCIENT SUMER' \
    'FIVE YEAR TERM' \
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
