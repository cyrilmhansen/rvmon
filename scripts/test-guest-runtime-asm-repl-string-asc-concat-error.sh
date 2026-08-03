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
printf '%s\n' '10 PRINT ASC("A"+"")' '20 PRINT ASC("")' '30 END' 'RUN' |
    while IFS= read -r line; do
        printf '%s\n' "$line" >&3
        sleep 0.15
    done
sleep 0.8
printf 'q\n' >&3
exec 3>&-
sleep 0.2
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

if (( $(grep -aF -- 'ERR' "$output_file" | wc -l) < 2 )) ||
    ! grep -aFq -- 'trap: breakpoint' "$output_file"; then
    cat "$output_file"
    printf 'ASC string-concatenation error test failed\n' >&2
    exit 1
fi
printf 'guest assembly REPL ASC string-concatenation error QEMU test passed\n'
