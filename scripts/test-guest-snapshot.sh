#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null

workspace_start_hex="$(riscv64-linux-gnu-nm -n target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor |
    awk '$3 == "_target_workspace_start" && !found { print $1; found=1 }')"
data_start_hex="$(riscv64-linux-gnu-nm -n target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor |
    awk '$3 == "_target_data_start" && !found { print $1; found=1 }')"
if [[ -z "$workspace_start_hex" || -z "$data_start_hex" ]]; then
    printf 'could not locate guest symbols\n' >&2
    exit 1
fi

assembly_address="$(printf '0x%x' "$((16#$workspace_start_hex + 0x240))")"
data_address="$(printf '0x%x' "$((16#$data_start_hex + 0x70))")"
data_address_full="$(printf '0x%016x' "$((16#$data_start_hex + 0x70))")"

set +e
output="$({
    printf 'assemble-program %s\naddi x1,x0,7\nend\n' "$assembly_address"
    printf 'set x1 0x7\ndata %s .word 0x11223344\nsnapshot save\nsnapshot info\nsnapshot manifest\nsnapshot dump data 112 4\nsnapshot patch data 112 aabbccdd\nsnapshot manifest\nsnapshot dump data 112 4\nsnapshot dump data 1048576 1\nsnapshot patch data 0 abc\nset x1 0x99\nedit %s deadbeef\nsource replace 1 "addi x1,x0,9"\n' "$data_address" "$data_address"
    printf 'snapshot restore\nregs\nmemory %s 4\nsource 1\nproject-save\nset x1 0x88\nproject-load\nregs\n' "$data_address"
    printf 'memory %s 4\nsource 1\nquit\n' "$data_address"
} | timeout 5s qemu-system-riscv64 \
    -M virt \
    -m 64M \
    -bios none \
    -kernel target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor \
    -nographic 2>&1)"
status=$?
set -e

if [[ "$status" -ne 124 ]]; then
    printf '%s\n' "$output"
    printf 'guest snapshot test exited with status %s, expected timeout 124\n' "$status" >&2
    exit 1
fi

for expected in \
    'snapshot saved (workspace=65536 data=1048576)' \
    'snapshot: valid workspace=65536 data=1048576 source-lines=1 chunk-max=32' \
    'snapshot-manifest format=RVSNAP01 workspace-size=65536 data-size=1048576 source-lines=1' \
    'snapshot-chunk data offset=112 length=4 hex=44332211' \
    'snapshot chunk patched data offset=112 length=4' \
    'snapshot-chunk data offset=112 length=4 hex=aabbccdd' \
    'snapshot restored (workspace=65536 data=1048576)' \
    'set x1=0x0000000000000099' \
    'x1=0x0000000000000007' \
    "$data_address_full: aa bb cc dd" \
    '1 | addi x1,x0,7'; do
    if ! [[ "$output" == *"$expected"* ]]; then
        printf '%s\n' "$output"
        printf 'missing expected output: %s\n' "$expected" >&2
        exit 1
    fi
done

manifest_count="$(printf '%s\n' "$output" | grep -c '^snapshot-manifest ')"
if [[ "$manifest_count" -ne 2 ]]; then
    printf '%s\n' "$output"
    printf 'expected two snapshot manifests, found %s\n' "$manifest_count" >&2
    exit 1
fi
first_data_crc="$(printf '%s\n' "$output" | grep '^snapshot-manifest ' | sed -n '1p' | sed -E 's/.*data-crc32=(0x[0-9a-f]+).*/\1/')"
second_data_crc="$(printf '%s\n' "$output" | grep '^snapshot-manifest ' | sed -n '2p' | sed -E 's/.*data-crc32=(0x[0-9a-f]+).*/\1/')"
if [[ -z "$first_data_crc" || "$first_data_crc" == "$second_data_crc" ]]; then
    printf '%s\n' "$output"
    printf 'snapshot data CRC did not change after patch\n' >&2
    exit 1
fi
for expected in \
    'error [GUEST-SNAPSHOT-007]: snapshot chunk must be 1..32 bytes inside its region' \
    'error [GUEST-SNAPSHOT-009]: snapshot patch expects 1..32 hexadecimal bytes'; do
    if ! [[ "$output" == *"$expected"* ]]; then
        printf '%s\n' "$output"
        printf 'missing expected output: %s\n' "$expected" >&2
        exit 1
    fi
done

printf 'guest snapshot/project QEMU test passed\n'
