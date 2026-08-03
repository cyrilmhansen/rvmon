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
sleep 0.2
printf '%s\n' \
  'assemble-program 0x81000100' \
  'ebreak' \
  'addi x1,x0,1' \
  'jal x0,target' \
  'addi x3,x1,2' \
  'target:' \
  'fadd.d f3,f0,f0' \
  'end' \
  'run-at 0x81000100' >&3
sleep 1
printf '%s\n' 'stepidp 3' >&3
sleep 2
printf 'q\n' >&3
exec 3>&-
sleep 0.2
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

for expected in 'stepidp retired pc=' 'fcsr=0x' 'floating registers (raw bits):' 'stepidp stack['; do
    if ! grep -aFq -- "$expected" "$output_file"; then
        cat "$output_file"
        printf 'missing stepidp evidence: %s\n' "$expected" >&2
        exit 1
    fi
done
if [[ "$(grep -ac 'stepidp retired pc=' "$output_file")" -ne 3 ]]; then
    cat "$output_file"
    printf 'stepidp did not report exactly three retired instructions\n' >&2
    exit 1
fi
printf 'guest monitor stepidp QEMU test passed\n'
