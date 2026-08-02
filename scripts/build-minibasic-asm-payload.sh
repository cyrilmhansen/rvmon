#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

assembler="${RISCV_AS:-riscv64-linux-gnu-as}"
objcopy="${RISCV_OBJCOPY:-riscv64-linux-gnu-objcopy}"
objdump="${RISCV_OBJDUMP:-riscv64-linux-gnu-objdump}"
payload_source="examples/minibasic-asm/payload-repl.rv"
output_dir="${MINIBASIC_ASM_OUTPUT_DIR:-target/payloads}"
mkdir -p "$output_dir"
temporary_dir="$(mktemp -d)"
trap 'rm -rf "$temporary_dir"' EXIT

assembly_source="$temporary_dir/minibasic-payload.s"
object_file="$temporary_dir/minibasic-payload.o"
binary_file="$output_dir/minibasic-payload-asm.bin"
listing_file="$output_dir/minibasic-payload-asm.objdump.txt"
data_source="$temporary_dir/minibasic-payload-data.s"
data_object="$temporary_dir/minibasic-payload-data.o"
data_binary="$output_dir/minibasic-payload-asm-data.bin"

awk '
    /^assemble-program / { active = 1; next }
    active && /^end$/ { exit }
    active { print }
' "$payload_source" |
    sed 's/;.*/\n/' |
    sed -E \
        -e 's/^(fcvt[.]d[.]l [^,]+,[^,]+),0$/\1,rne/' \
        -e 's/^(fcvt[.]l[.]d [^,]+,[^,]+),1$/\1,rtz/' \
    > "$assembly_source"

"$assembler" \
    -march=rv64imafd_zicsr_zifencei \
    -mabi=lp64d \
    -o "$object_file" \
    "$assembly_source"
"$objcopy" -O binary --only-section=.text "$object_file" "$binary_file"
"$objdump" -dr "$object_file" > "$listing_file"

{
    printf '.section .data\n'
    while read -r _ address directive value; do
        offset=$((address - 0x82000000))
        printf '.org 0x%x\n%s %s\n' "$offset" "$directive" "$value"
    done < <(awk '$1 == "data" { print }' "$payload_source" | sort -k2,2)
} > "$data_source"
"$assembler" -march=rv64imafd_zicsr_zifencei -mabi=lp64d -o "$data_object" "$data_source"
"$objcopy" -O binary --only-section=.data "$data_object" "$data_binary"

size_bytes="$(wc -c < "$binary_file")"
if (( size_bytes == 0 || size_bytes > 0x10000 )); then
    printf 'MiniBASIC assembly payload size %d is outside the 64 KiB workspace\n' \
        "$size_bytes" >&2
    exit 1
fi

printf 'MiniBASIC assembly payload built: %d bytes\n' "$size_bytes"
printf '  binary:  %s\n' "$binary_file"
printf '  listing: %s\n' "$listing_file"
printf '  data:    %s\n' "$data_binary"
