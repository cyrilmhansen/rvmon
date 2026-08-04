#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

target="riscv64gc-unknown-none-elf"
payload_dir="target/payloads"
mkdir -p "$payload_dir"

RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=-c -C link-arg=$repo_root/crates/minibasic-payload/linker.ld" \
    cargo build -p luna-minibasic-payload --release --target "$target" >/dev/null

elf="target/$target/release/luna-minibasic-payload"
binary="$payload_dir/minibasic-payload.bin"
listing="$payload_dir/minibasic-payload.objdump.txt"
symbols="$payload_dir/minibasic-payload.nm.txt"

riscv64-linux-gnu-objcopy -O binary "$elf" "$binary"
riscv64-linux-gnu-objdump -dr "$elf" >"$listing"
riscv64-linux-gnu-nm -n "$elf" >"$symbols"

size_bytes="$(wc -c <"$binary")"
if (( size_bytes == 0 || size_bytes > 0x10000 )); then
    printf 'MiniBASIC payload size %d is outside the 64 KiB workspace\n' "$size_bytes" >&2
    exit 1
fi

printf 'MiniBASIC payload built: %d bytes\n' "$size_bytes"
printf '  binary:  %s\n' "$binary"
printf '  listing: %s\n' "$listing"
printf '  symbols: %s\n' "$symbols"
