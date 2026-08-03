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
    while IFS= read -r line; do
        [[ -z "$line" || "$line" == \;* ]] && continue
        printf '%s\n' "$line" >&3
        sleep 0.01
    done
sleep 0.3
printf '%s\n' \
  '10 PRINT TAN(0)' \
  '20 PRINT TAN(1)' \
  '30 PRINT TAN(-1)' \
  '40 PRINT TAN(3.141592653589793)' \
  '50 PRINT TAN(COS(0))' \
  '60 END' \
  'RUN' >&3
sleep 1.0
exec 3>&-
sleep 0.2
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

for expected in \
  '0.000000' \
  '1.557407' \
  '-1.557407' \
  '0.000000' \
  '1.557407' \
  'END' \
  'trap: breakpoint'; do
    if ! grep -aFq -- "$expected" "$output_file"; then
        cat "$output_file"
        printf 'missing tangent result: %s\n' "$expected" >&2
        exit 1
    fi
done
if grep -aFq -- 'ERR' "$output_file"; then
    cat "$output_file"
    printf 'unexpected tangent diagnostic\n' >&2
    exit 1
fi
printf 'guest assembly REPL TAN QEMU test passed\n'
