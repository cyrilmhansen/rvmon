#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

port=12351
qemu_log=${TMPDIR:-/tmp}/rvmonitor-qemu-gdb-$port.log

cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null
cargo build -p luna-qemu-backend --example qemu_probe >/dev/null

qemu-system-riscv64 \
    -M virt \
    -cpu max \
    -bios none \
    -kernel target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor \
    -nographic \
    -S \
    -gdb "tcp::$port" \
    >"$qemu_log" 2>&1 &
qemu_pid=$!
cleanup() {
    kill "$qemu_pid" 2>/dev/null || true
    wait "$qemu_pid" 2>/dev/null || true
}
trap cleanup EXIT

# Do not probe the port with a throw-away TCP connection: QEMU's GDB stub
# keeps the first client session and that would consume the only RSP peer.
sleep 0.2

probe_output=$(timeout 10s cargo run --quiet -p luna-qemu-backend --example qemu_probe -- "$port")
printf '%s\n' "$probe_output"
grep -q '^qemu-connect: pc=0x' <<<"$probe_output"
grep -q '^qemu-step: Stopped' <<<"$probe_output"

echo "QEMU GDB backend integration: PASS"
