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
    'TIME$="246000"' \
    'TIME$="010203"' \
    '10 PRINT TIME' \
    '20 PRINT TIME$' \
    '30 PRINT "T="+TIME$' \
    '40 LET TIME$="020304"' \
    '50 PRINT TIME$' \
    '60 PRINT "T="+TIME$' \
    '70 END' \
    'RUN' |
    while IFS= read -r line; do
        printf '%s\n' "$line" >&3
        sleep 0.15
    done
sleep 1.0
printf 'q\n' >&3
exec 3>&-
sleep 0.2
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

for expected in 'ERR' '1.000000' '010203' 'T=010203' '020304' 'T=020304' 'trap: breakpoint'; do
    if ! grep -aFq -- "$expected" "$output_file"; then
        cat "$output_file"
        printf 'missing TIME result: %s\n' "$expected" >&2
        exit 1
    fi
done
if grep -aFq -- 'mcause=' "$output_file"; then
    cat "$output_file"
    printf 'unexpected TIME target fault\n' >&2
    exit 1
fi
printf 'guest assembly REPL TIME QEMU test passed\n'
