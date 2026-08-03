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
sleep 15
printf '%s\n' \
    '10 PRINT HEX$(0)' \
    '20 PRINT HEX$(255)' \
    '30 PRINT HEX$(305441741)' \
    '40 LET H$=HEX$(255)+"!"' \
    '50 PRINT H$' \
    '60 END' \
    'RUN' \
    'PRINT HEX$(-1)' \
    'PRINT HEX$(1.5)' \
    'PRINT HEX$(4294967296)' |
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

for expected in '0' 'FF' '1234ABCD' 'FF!' 'trap: breakpoint'; do
    if ! grep -aFq -- "$expected" "$output_file"; then
        cat "$output_file"
        printf 'missing HEX$ result: %s\n' "$expected" >&2
        exit 1
    fi
done
if [[ "$(grep -aF -c 'error:' "$output_file" || true)" -lt 3 ]]; then
    cat "$output_file"
    printf 'HEX$ invalid-argument diagnostics are incomplete\n' >&2
    exit 1
fi
if grep -aFq -- 'mcause=' "$output_file"; then
    cat "$output_file"
    printf 'unexpected HEX$ target fault\n' >&2
    exit 1
fi
printf 'guest assembly REPL HEX$ QEMU test passed\n'
