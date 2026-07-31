#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

manifest_version() {
    local section="$1"
    awk -v wanted="[$section]" '
        $0 == wanted { in_section = 1; next }
        /^\[/ { in_section = 0 }
        in_section && /^version = / {
            gsub(/"/, "", $3)
            print $3
            exit
        }
    ' norms/oracles/manifest.toml
}

gnu_version="$(riscv64-linux-gnu-as --version | awk 'NR == 1 { print $NF }')"
llvm_version="$(llvm-mc --version | awk '$1 == "LLVM" && $2 == "version" { print $3; exit }')"
expected_gnu="$(manifest_version gnu)"
expected_llvm="$(manifest_version llvm)"
if [[ "$gnu_version" != "$expected_gnu" ]]; then
    printf 'GNU oracle version mismatch: expected %s, found %s\n' "$expected_gnu" "$gnu_version" >&2
    exit 1
fi
if [[ "$llvm_version" != "$expected_llvm" ]]; then
    printf 'LLVM oracle version mismatch: expected %s, found %s\n' "$expected_llvm" "$llvm_version" >&2
    exit 1
fi

temp_dir="$(mktemp -d)"
trap 'rm -rf "$temp_dir"' EXIT
source_file="$temp_dir/corpus.s"
gnu_object="$temp_dir/gnu.o"
gnu_binary="$temp_dir/gnu.bin"
llvm_object="$temp_dir/llvm.o"
llvm_binary="$temp_dir/llvm.bin"

awk -F'|' '!/^#/ && NF == 2 { print $1 }' tests/golden/r1-encoding-corpus.tsv > "$source_file"
expected_hex="$(awk -F'|' '!/^#/ && NF == 2 { gsub(/[[:space:]]/, "", $2); printf "%s", $2 }' tests/golden/r1-encoding-corpus.tsv)"

riscv64-linux-gnu-as \
    -march=rv64imafd_zicsr_zifencei \
    -mabi=lp64d \
    -o "$gnu_object" "$source_file"
riscv64-linux-gnu-objcopy -j .text -O binary "$gnu_object" "$gnu_binary"

llvm-mc \
    -triple riscv64 \
    -mattr=+m,+f,+d,+zicsr,+zifencei \
    -filetype=obj \
    -o "$llvm_object" "$source_file"
llvm-objcopy -j .text -O binary "$llvm_object" "$llvm_binary"

actual_gnu="$(od -An -tx1 -v "$gnu_binary" | tr -d '[:space:]')"
actual_llvm="$(od -An -tx1 -v "$llvm_binary" | tr -d '[:space:]')"
if [[ "$actual_gnu" != "$expected_hex" ]]; then
    printf 'GNU encoding differs from the R1 golden corpus\nexpected: %s\nactual:   %s\n' \
        "$expected_hex" "$actual_gnu" >&2
    exit 1
fi
if [[ "$actual_llvm" != "$expected_hex" ]]; then
    printf 'LLVM encoding differs from the R1 golden corpus\nexpected: %s\nactual:   %s\n' \
        "$expected_hex" "$actual_llvm" >&2
    exit 1
fi
if [[ "$actual_gnu" != "$actual_llvm" ]]; then
    printf 'GNU and LLVM encodings differ\nGNU:   %s\nLLVM:  %s\n' "$actual_gnu" "$actual_llvm" >&2
    exit 1
fi

printf 'GNU %s and LLVM %s agree on %s independent R1 encodings: PASS\n' \
    "$gnu_version" "$llvm_version" "$(awk -F'|' '!/^#/ && NF == 2 { count++ } END { print count + 0 }' tests/golden/r1-encoding-corpus.tsv)"
