#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null

target_entry_hex="$(riscv64-linux-gnu-nm -n target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor |
    awk '$3 == "target_entry" && !found { print $1; found=1 }')"
workspace_start_hex="$(riscv64-linux-gnu-nm -n target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor |
    awk '$3 == "_target_workspace_start" && !found { print $1; found=1 }')"
data_start_hex="$(riscv64-linux-gnu-nm -n target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor |
    awk '$3 == "_target_data_start" && !found { print $1; found=1 }')"
if [[ -z "$target_entry_hex" || -z "$workspace_start_hex" || -z "$data_start_hex" ]]; then
    printf 'could not locate guest symbols\n' >&2
    exit 1
fi

breakpoint_address="$(printf '0x%x' "$((16#$target_entry_hex + 12))")"
assembly_address="$(printf '0x%x' "$((16#$workspace_start_hex + 0x100))")"
assembly_address_full="$(printf '0x%016x' "$((16#$workspace_start_hex + 0x100))")"
edit_address="$(printf '0x%x' "$((16#$workspace_start_hex + 0x40))")"
edit_address_full="$(printf '0x%016x' "$((16#$workspace_start_hex + 0x40))")"
data_address="$(printf '0x%x' "$((16#$data_start_hex + 0x60))")"
data_address_full="$(printf '0x%016x' "$((16#$data_start_hex + 0x60))")"

set +e
output="$({
    printf 'assemble %s ld x3,-8(x4)\ndisasm %s 1\n' "$assembly_address" "$assembly_address"
    printf 'edit 0x84000000 0000\n'
    printf 'help\nregs\nmemory 0x80000000 16\nbreak %s\ninfo break\ncontinue\nregs\ncontinue\ndelete 1\nstep\nregs\nstep\nregs\nassemble-program %s\n_start:\naddi x1,x0,1\nbeq x1,x1,next\naddi x1,x0,99\nnext:\nfadd.s f3,f1,f2\nfadd.d f6,f4,f5\nfmv.w.x f7,x1\nfmv.x.w x8,f7\naddi x1,x1,2\nend\nedit %s deadbeef\nmemory %s 4\nundo\nmemory %s 4\nedit 0x8001ffff 0000\ndata %s .word 0x11223344\nmemory %s 4\nundo\ndata %s .float 0x3f800000\nmemory %s 4\nundo\ndata %s .binary128 000102030405060708090a0b0c0d0e0f\nmemory %s 16\nundo\nsymbols\ndisasm _start 8\nsetf f1 0xffffffff3f800000\nsetf f2 0xffffffff40000000\nsetf f4 0x3ff0000000000000\nsetf f5 0x4000000000000000\nbreak next\ninfo break\ndelete 1\nstep\nregs\nstep\nregs\nstep\nregs\nstep\nregs\nstep\nregs\nstep\nregs\nstep\nregs\nquit\n' \
        "$breakpoint_address" "$assembly_address" "$edit_address" "$edit_address" "$edit_address" \
        "$data_address" "$data_address" "$data_address" "$data_address" "$data_address" "$data_address"
    printf 'assemble-program %s\naddi x1,x0,7\nnot-an-instruction x1\nend\ndisasm %s 1\n' \
        "$assembly_address" "$assembly_address"
    printf 'set x9 0x8000000080000000\nset x0 0x1\n'
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
    printf 'guest monitor exited with status %s, expected timeout 124\n' "$status" >&2
    exit 1
fi

for expected in \
    'trap: breakpoint' \
    'breakpoint #1 set at' \
    'breakpoints:' \
    'breakpoint #1 deleted' \
    'integer registers:' \
    'set x9=0x8000000080000000' \
    'error: x0 is read-only' \
    'floating registers (raw bits):' \
    '0x0000000080000000:' \
    'assembled instruction at' \
    'ld x3,-8(x4)' \
    'x31=0x' \
    'f31=0x' \
    'source mode: enter integer/control, ld/sd, fadd.s/fadd.d or fmv lines, finish with end' \
    "assembled program: 8 instruction(s) at $assembly_address_full" \
    "edited 4 byte(s) at $edit_address_full" \
    "$edit_address_full: de ad be ef" \
    "undone 4 byte(s) at $edit_address_full" \
    "$edit_address_full: 00 00 00 00" \
    "stored .word at $data_address_full (4 byte(s))" \
    "$data_address_full: 44 33 22 11" \
    "undone 4 byte(s) at $data_address_full" \
    "stored .float at $data_address_full (4 byte(s))" \
    "$data_address_full: 00 00 80 3f" \
    "stored .binary128 at $data_address_full (16 byte(s))" \
    "$data_address_full: 0f 0e 0d 0c 0b 0a 09 08 07 06 05 04 03 02 01 00" \
    '_start' \
    'next' \
    'addi x1,x0,1' \
    'beq x1,x1,next' \
    'addi x1,x1,2' \
    'fadd.s f3,f1,f2' \
    'fadd.d f6,f4,f5' \
    'fmv.w.x f7,x1' \
    'fmv.x.w x8,f7' \
    'set f1=0xffffffff3f800000' \
    'set f5=0x4000000000000000' \
    'f3=0xffffffff40400000' \
    'f6=0x4008000000000000' \
    'f7=0xffffffff00000001' \
    'x8=0x0000000000000001' \
    'fcsr=0x0000000000000000' \
    'error [GUEST-ASM-008] source line 2:' \
    'addi x1,x0,1' \
    'breakpoint #1 set at' \
    'breakpoint #1 deleted' \
    'x1=0x0000000000000003'; do
    if ! [[ "$output" == *"$expected"* ]]; then
        printf '%s\n' "$output"
        printf 'missing expected output: %s\n' "$expected" >&2
        exit 1
    fi
done

printf 'guest monitor QEMU end-to-end test passed\n'
