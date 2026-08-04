#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null
bash scripts/build-minibasic-payload.sh >/dev/null

image=target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor
payload=target/payloads/minibasic-payload.bin
payload_symbols=target/payloads/minibasic-payload.nm.txt
payload_size="$(wc -c <"$payload")"
payload_start=0x81000000
payload_entry="$(awk '$3 == "_start" { print $1; exit }' "$payload_symbols")"
if [[ -z "$payload_entry" ]]; then
    printf 'missing _start in payload symbol map\n' >&2
    exit 1
fi

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
python3 - "$payload" "$payload_start" <<'PY' >&3
import sys
import time

path, start = sys.argv[1], int(sys.argv[2], 0)
with open(path, "rb") as stream:
    offset = 0
    while True:
        block = stream.read(32)
        if not block:
            break
        command = f"payload-load 0x{start + offset:x} {block.hex()}\n".encode()
        sys.stdout.buffer.write(command)
        sys.stdout.buffer.flush()
        offset += len(block)
        time.sleep(0.005)
PY

sleep 0.5

printf 'run-at 0x%s\n' "$payload_entry" >&3
printf 'PRINT 2+3*4\nPRINT 22/7\nBYE\n' >&3
exec 3>&-
sleep 0.8
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""
output="$(<"$output_file")"

for expected in \
    'MiniBASIC-RV' \
    'READY>' \
    '14.000000' \
    '3.142857' \
    'target exit status=0'; do
    if [[ "$output" != *"$expected"* ]]; then
        printf '%s\n' "$output"
        printf 'missing expected loaded MiniBASIC output: %s\n' "$expected" >&2
        exit 1
    fi
done

printf 'guest loaded MiniBASIC payload QEMU test passed (%d bytes)\n' "$payload_size"
