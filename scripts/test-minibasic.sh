#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null
image=target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor

set +e
output="$({
    sleep 0.1
    printf 'basic\n'
    printf 'PRINT 2+3*4\nPRINT 22/7\nPOPULATION=95\nPRINT population\nABCDEFGHIJKLMNOP=7\nPRINT abcdefghijklmnop\nABCDEFGHIJKLMNOPQ=8\n'
    printf '10 PRINT "OLD"\n10 PRINT "NEW"\n10\nLIST\n'
    printf '40 PRINT I,X\n10 PRINT "RV64 MINIBASIC"\n60 END\n30 X=I/3\n20 FOR I=1 TO 10\n50 NEXT I\nLIST\nDUMP\nTRACE ON\nRUN\n'
    printf '10 PRINT "BEFORE"\n20 GOTO 999\n30 END\nRUN\nBYE\n'
    printf 'basic\n10 INPUT N\n20 IF N<0 THEN 50\n30 PRINT N*N\n40 GOTO 10\n50 END\nRUN\n3\n4\n-1\nBYE\nq\n'
    printf 'basic\n10 GOTO 10\nRUN\n'
    sleep 0.2
    printf '\003'
    sleep 0.2
    printf '\nBYE\nq\n'
} | timeout 12s qemu-system-riscv64 -M virt -m 64M -bios none -kernel "$image" -nographic 2>&1)"
status=$?
set -e

if [[ -n "${MINIBASIC_TRANSCRIPT:-}" ]]; then
    mkdir -p "$(dirname -- "$MINIBASIC_TRANSCRIPT")"
    printf '%s\n' "$output" > "$MINIBASIC_TRANSCRIPT"
fi

if [[ "$status" -ne 124 ]]; then
    printf '%s\n' "$output"
    printf 'MiniBASIC QEMU test exited with status %s, expected timeout 124\n' "$status" >&2
    exit 1
fi
for expected in \
    'MiniBASIC-RV' \
    '14.000000' \
    '3.142857' \
    '95.000000' \
    '7.000000' \
    'ERROR [BASIC-SYNTAX-001]' \
    '10 PRINT "NEW"' \
    '10 PRINT "RV64 MINIBASIC"' \
    '1.000000 0.333333' \
    '10.000000 3.333333' \
    'slot=0 address=0x' \
    '[10]' \
    'ERROR [BASIC-FLOW-001] line=20' \
    '9.000000' \
    '16.000000' \
    'ERROR [BASIC-RUN-001]' \
    'target exit status=0'; do
    if [[ "$output" != *"$expected"* ]]; then
        printf '%s\n' "$output"
        printf 'missing expected output: %s\n' "$expected" >&2
        exit 1
    fi
done
printf 'MiniBASIC-RV target QEMU test passed\n'
