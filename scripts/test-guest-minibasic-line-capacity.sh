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
    for _ in {1..600}; do
        if grep -aFq -- "$text" "$output_file"; then return 0; fi
        sleep 0.01
    done
    cat "$output_file" >&2
    printf 'timeout waiting for guest text: %s\n' "$text" >&2
    return 1
}

sleep 0.2
printf 'basic\n' >&3
wait_for_text 'READY> '

printf '2550 GOTO 2560\n' >&3
printf '2560 PRINT 7\n' >&3
printf '2570 PRINT 9\n' >&3
wait_for_text 'ERR'
wait_for_text 'READY> '

printf 'LIST\n' >&3
wait_for_text '2560'
printf 'RUN\n' >&3
wait_for_text '7.000000'
wait_for_text 'trap: breakpoint'

exec 3>&-
sleep 0.2
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

if grep -aFq -- 'mcause=0x0000000000000005' "$output_file" || \
   grep -aFq -- 'error: unknown command' "$output_file"; then
    cat "$output_file"
    printf 'MiniBASIC line-capacity test faulted or delegated a command\n' >&2
    exit 1
fi
printf 'guest MiniBASIC 2560-line boundary QEMU test passed\n'
