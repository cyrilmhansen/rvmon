#!/usr/bin/env bash
set -euo pipefail

transcript="${1:?usage: check-tutorial-transcript.sh <transcript.txt>}"
test -s "$transcript"

must_contain() {
    local needle="$1"
    if ! grep -aFq -- "$needle" "$transcript"; then
        printf 'tutorial transcript missing: %s\n' "$needle" >&2
        exit 1
    fi
}

must_not_contain() {
    local needle="$1"
    if grep -aFq -- "$needle" "$transcript"; then
        printf 'tutorial transcript contains forbidden failure: %s\n' "$needle" >&2
        exit 1
    fi
}

must_contain 'assembled instruction at 0x0000000081000100 = 0x0000000000100093'
must_contain 'assembled program: 5 instruction(s) at 0x0000000081000a30'
must_contain 'error [GUEST-ASM-008] source line 2'
must_contain 'source line 2 updated; use assemble-source to apply'
must_contain 'f3=0xffffffff40400000'
must_contain 'f6=0x4008000000000000'
must_contain 'snapshot saved (workspace=65536 data=1048576)'
must_contain 'snapshot restored (workspace=65536 data=1048576)'
must_contain 'watchpoint #1 set at 0x0000000082000060 width=8 mode=write'
must_contain 'MiniBASIC-RV'
must_contain 'TRACE ON'
must_contain '[10]'
must_contain 'HAMMURABI-RV'
must_contain 'DUMP'

must_not_contain 'error: unknown command; use help'
must_not_contain 'error: target is not stopped at a breakpoint'
must_not_contain 'mcause=0x0000000000000002'
must_not_contain 'trap: illegal'

printf 'tutorial transcript validation passed: %s\n' "$transcript"
