#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null
bash scripts/build-minibasic-asm-payload.sh >/dev/null
image=target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor
binary=target/payloads/minibasic-payload-asm.bin
data_binary=target/payloads/minibasic-payload-asm-data.bin

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

load_region() {
    local command="$1"
    local address="$2"
    local file="$3"
    while IFS= read -r bytes; do
        printf '%s 0x%08x %s\n' "$command" "$address" "$bytes" >&3
        sleep 0.002
        address=$((address + ${#bytes} / 2))
    done < <(xxd -p -c32 "$file")
}

sleep 0.2
load_region payload-load 0x81000100 "$binary"
load_region payload-load-data 0x82000000 "$data_binary"

printf 'run-at 0x81000100\n' >&3
wait_for_text 'READY> '
printf '%s\n' \
  'LET TEXT$="HAMMURABI"' \
  'LET LONGTEXT$="NILE"' \
  'LET COPIED$=LONGTEXT$' \
  'LET LONGTEXT2$=LONGTEXT$+" RIVER"' \
  'LET NUMSTR$=STR$(12.5)' \
  'LET SIGNEDSTR$="N="+STR$(-3.25)' \
  'LET CHAR$=CHR$(65)' \
  'LET CHARS$=CHAR$+CHR$(66)' \
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
  '90 PRINT CHAR$' \
  '100 PRINT CHARS$' \
  '110 PRINT ASC(TEXT$)' \
  '120 PRINT ASC(A$(0))' \
  '130 PRINT CHR$(67)' \
  '140 PRINT VAL("12.5")' \
  '150 LET NUMTEXT$="3.25"' \
  '160 PRINT VAL(NUMTEXT$)' \
  '170 PRINT INSTR("HAMMURABI","MUR")' \
  '180 LET NEEDLE$="RABI"' \
  '190 PRINT INSTR(TEXT$,NEEDLE$)' \
  '200 PRINT INSTR(TEXT$,"XYZ")' \
  '210 PRINT INSTR(TEXT$,"")' \
  '220 PRINT NUMSTR$' \
  '230 PRINT SIGNEDSTR$' \
  '240 PRINT STR$(7.25)' \
  '250 END' \
  'RUN' >&3
wait_for_text '7.250000'
wait_for_text 'trap: breakpoint'
printf 'q\n' >&3
exec 3>&-
sleep 0.2
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

for expected in \
  'RV HAMMURABI' 'RV HAMMURABI!' 'HAMMURABI GAME' '>HAMM' 'RABI<' '[AMMU]' \
  'NILE' 'NILE RIVER' 'A' 'AB' '72.000000' 'C' '12.500000' '3.250000' \
  '4.000000' '6.000000' '0.000000' '1.000000' 'N=-3.250000' '7.250000'; do
    if ! grep -aFq -- "$expected" "$output_file"; then
        cat "$output_file"
        printf 'missing string-concat result: %s\n' "$expected" >&2
        exit 1
    fi
done
if grep -aFq -- 'error: unknown command' "$output_file" ||
   grep -aFq -- 'mcause=0x0000000000000002' "$output_file"; then
    cat "$output_file"
    printf 'unexpected string-concat payload failure\n' >&2
    exit 1
fi
printf 'guest assembly REPL string concatenation QEMU test passed\n'
