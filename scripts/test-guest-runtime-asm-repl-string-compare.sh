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
        sleep 0.01
    done
assembled=0
for _ in $(seq 1 600); do
    if grep -aFq -- 'READY>' "$output_file"; then
        assembled=1
        break
    fi
    if grep -aFq -- 'GUEST-ASM-' "$output_file"; then
        cat "$output_file"
        printf 'guest payload assembly failed\n' >&2
        exit 1
    fi
    sleep 1
done
if [[ "$assembled" -ne 1 ]]; then
    cat "$output_file"
    printf 'timed out waiting for guest payload assembly\n' >&2
    exit 1
fi
printf '%s\n' \
    '10 IF "A"="A" THEN 30' \
    '20 PRINT "BAD-EQ"' \
    '30 PRINT "EQ-OK"' \
    '40 LET TEXT$="HAM"' \
    '50 IF TEXT$<>"MUR" THEN 70' \
    '60 PRINT "BAD-NE"' \
    '70 PRINT "NE-OK"' \
    '80 IF "A"<"B" THEN 100' \
    '90 PRINT "BAD-LT"' \
    '100 PRINT "LT-OK"' \
    '110 IF "A"<="A" THEN 130' \
    '120 PRINT "BAD-LE"' \
    '130 PRINT "LE-OK"' \
    '140 IF "B">"A" THEN 160' \
    '150 PRINT "BAD-GT"' \
    '160 PRINT "GT-OK"' \
    '170 IF "B">="B" THEN 190' \
    '180 PRINT "BAD-GE"' \
    '190 PRINT "GE-OK"' \
    '200 END' \
    'RUN' |
    while IFS= read -r line; do
        printf '%s\n' "$line" >&3
        sleep 0.12
    done
sleep 1.0
printf 'q\n' >&3
exec 3>&-
sleep 0.2
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

for expected in 'EQ-OK' 'NE-OK' 'LT-OK' 'LE-OK' 'GT-OK' 'GE-OK' 'trap: breakpoint'; do
    if ! grep -aFq -- "$expected" "$output_file"; then
        cat "$output_file"
        printf 'missing string-comparison result: %s\n' "$expected" >&2
        exit 1
    fi
done
for forbidden in 'BAD-EQ' 'BAD-NE' 'BAD-LT' 'BAD-LE' 'BAD-GT' 'BAD-GE'; do
    if grep -aFq -- "$forbidden" "$output_file"; then
        cat "$output_file"
        printf 'unexpected false string-comparison branch: %s\n' "$forbidden" >&2
        exit 1
    fi
done
if grep -aFq -- 'ERR' "$output_file" || grep -aFq -- 'mcause=' "$output_file"; then
    cat "$output_file"
    printf 'unexpected string-comparison diagnostic or target fault\n' >&2
    exit 1
fi
printf 'guest assembly REPL string comparison QEMU test passed\n'
