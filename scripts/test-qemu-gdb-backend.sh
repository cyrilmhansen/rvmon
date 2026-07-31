#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

port=12351
qemu_log=${TMPDIR:-/tmp}/rvmonitor-qemu-gdb-$port.log

cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null
cargo build -p luna-app >/dev/null

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

console_output=$(printf 'regs\nmemory 0x80000000 4\nstep\nquit\n' \
    | timeout 10s target/debug/luna-app --qemu-port "$port")
printf '%s\n' "$console_output"
grep -q '^RVMonitor QEMU backend on ' <<<"$console_output"
grep -q '^pc=0x' <<<"$console_output"
grep -q '^0x0000000080000000:' <<<"$console_output"
grep -q '^0x0000000000001000:' <<<"$console_output"
grep -q 'stopped: Breakpoint' <<<"$console_output"

echo "QEMU GDB backend integration: PASS"
