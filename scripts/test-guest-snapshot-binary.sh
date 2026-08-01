#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

port=12355
qemu_log="$(mktemp /tmp/rvmonitor-guest-binary-qemu.XXXXXX)"
cleanup() {
    if [[ -n "${qemu_pid:-}" ]]; then
        kill "$qemu_pid" 2>/dev/null || true
        wait "$qemu_pid" 2>/dev/null || true
    fi
    rm -f "$qemu_log"
}
trap cleanup EXIT

cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null
qemu-system-riscv64 \
    -M virt \
    -m 64M \
    -bios none \
    -kernel target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor \
    -display none \
    -serial "tcp:127.0.0.1:${port},server=on,wait=on" \
    >"$qemu_log" 2>&1 &
qemu_pid=$!

python3 - "$port" <<'PY'
import socket
import sys
import time

port = int(sys.argv[1])
prompt = b"rvmonitor> "

for _ in range(50):
    try:
        sock = socket.create_connection(("127.0.0.1", port), timeout=1)
        break
    except OSError:
        time.sleep(0.1)
else:
    raise SystemExit("cannot connect to guest UART")

sock.settimeout(5)

def receive_until(marker):
    data = bytearray()
    while marker not in data:
        byte = sock.recv(1)
        if not byte:
            raise SystemExit("guest UART closed")
        data.extend(byte)
        if len(data) > 2 * 1024 * 1024:
            raise SystemExit("guest UART response too large")
    return bytes(data)

receive_until(prompt)
sock.sendall(b"snapshot save\n")
receive_until(prompt)
sock.sendall(b"snapshot patchbin data 112 4\n")
receive_until(b"snapshot binary ready\r\n")
sock.sendall(b"\x01\x02\x03\x04")
receive_until(prompt)
sock.sendall(b"snapshot dump data 112 4\n")
response = receive_until(prompt)
if b"snapshot-chunk data offset=112 length=4 hex=01020304" not in response:
    raise SystemExit(response.decode("utf-8", "replace"))
sock.sendall(b"quit\n")
sock.close()
PY

printf 'guest binary snapshot patch TCP test passed\n'
