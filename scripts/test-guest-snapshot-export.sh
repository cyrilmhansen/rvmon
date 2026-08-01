#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

port=12353
snapshot_file="$(mktemp /tmp/rvmonitor-guest-snapshot.XXXXXX)"
qemu_log="$(mktemp /tmp/rvmonitor-guest-snapshot-qemu.XXXXXX)"
cleanup() {
    if [[ -n "${qemu_pid:-}" ]]; then
        kill "$qemu_pid" 2>/dev/null || true
        wait "$qemu_pid" 2>/dev/null || true
    fi
    rm -f "$snapshot_file" "$qemu_log"
}
trap cleanup EXIT

cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null
cargo build -p luna-app >/dev/null
qemu-system-riscv64 \
    -M virt \
    -m 64M \
    -bios none \
    -kernel target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor \
    -display none \
    -serial "tcp:127.0.0.1:${port},server=on,wait=on" \
    >"$qemu_log" 2>&1 &
qemu_pid=$!

output="$(timeout 90s cargo run -p luna-app --quiet -- \
    --guest-uart-port "$port" \
    --snapshot-out "$snapshot_file" 2>&1)"
printf '%s\n' "$output"

if [[ ! -s "$snapshot_file" ]]; then
    printf 'snapshot export produced no file\n' >&2
    exit 1
fi
magic="$(od -An -tc -N8 "$snapshot_file" | tr -d '[:space:]')"
if [[ "$magic" != "RVSNAP01" ]]; then
    printf 'unexpected snapshot magic: %s\n' "$magic" >&2
    exit 1
fi
expected_size=$((32 + 0x10000 + 0x100000))
actual_size="$(stat -c '%s' "$snapshot_file")"
if [[ "$actual_size" -ne "$expected_size" ]]; then
    printf 'unexpected snapshot size: %s (expected %s)\n' "$actual_size" "$expected_size" >&2
    exit 1
fi
[[ "$output" == *"guest snapshot exported"* ]]
printf 'guest snapshot TCP export test passed\n'
