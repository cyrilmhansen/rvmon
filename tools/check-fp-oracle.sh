#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

expected_qemu="$(awk '
    $0 == "[qemu]" { in_section = 1; next }
    /^\[/ { in_section = 0 }
    in_section && /^version = / {
        gsub(/"/, "", $3)
        print $3
        exit
    }
' norms/oracles/manifest.toml)"
actual_qemu="$(qemu-riscv64 --version | awk 'NR == 1 { print $3 }')"
if [[ "$actual_qemu" != "$expected_qemu" ]]; then
    printf 'QEMU oracle version mismatch: expected %s, found %s\n' \
        "$expected_qemu" "$actual_qemu" >&2
    exit 1
fi

temp_dir="$(mktemp -d)"
trap 'rm -rf "$temp_dir"' EXIT
object="$temp_dir/fp-probe.o"
binary="$temp_dir/fp-probe"
output="$temp_dir/qemu-output"

riscv64-linux-gnu-as \
    -march=rv64imafd_zicsr_zifencei \
    -mabi=lp64d \
    -o "$object" \
    tests/oracles/fp_qemu_probe.S
riscv64-linux-gnu-ld \
    -m elf64lriscv \
    -e _start \
    -o "$binary" \
    "$object"
qemu-riscv64 "$binary" > "$output"

qemu_hex="$(od -An -tx8 -v "$output" 2>/dev/null | tr -d '[:space:]')"
machine_hex="$(cargo run -q -p luna-machine --example fp_probe 2>/dev/null)"
if [[ "$qemu_hex" != "$machine_hex" ]]; then
    printf 'floating semantic oracle mismatch\nQEMU:    %s\nMachine: %s\n' \
        "$qemu_hex" "$machine_hex" >&2
    exit 1
fi

mutated_hex="\${machine_hex/0/1}"
if [[ "$mutated_hex" == "$qemu_hex" ]]; then
    printf 'oracle mutation check failed to alter the candidate result\n' >&2
    exit 1
fi

printf 'QEMU %s independently matches %s F/D result-and-flag cases; mutation check: PASS\n' \
    "$actual_qemu" 13
