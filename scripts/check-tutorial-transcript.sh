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
must_contain '=== MONITEUR GUEST — M-mode, cible U-mode et services console ==='
must_contain '=== ASM MINIBASIC-RV — aperçu source assembleur, non exécuté par l’hôte (500 lignes/s) ==='
must_contain '=== ASM MINIBASIC-RV — chargement binaire explicite via les commandes du moniteur ==='
must_contain 'asm-source> assemble-program 0x81000100'
must_contain 'payload loaded address=0x0000000081000100'
must_contain 'payload loaded address=0x0000000082000100'
must_contain '0x0000000082000100: 4d 49 4e 49 42 41 53 49 43 2d 52 56 20 41 53 4d'
must_contain 'MINIBASIC-RV ASM'
must_contain 'v0.3 2026-08-04'
must_contain '=== ASM MINIBASIC-RV — payload assembleur chargé et exécuté depuis le workspace ==='
must_contain '=== RUST MINIBASIC-RV — référence legacy résidente, non utilisée dans ce parcours ==='
must_contain '=== ASM MINIBASIC-RV — jeu final HAMMURABI-RV ==='
must_contain 'MiniBASIC-RV'
must_contain 'TRACE ON'
must_contain '[10]'
must_contain 'HAMMURABI-RV'
must_contain 'GOVERN SUMER'
must_contain 'ANCIENT SUMER'
must_contain 'FIVE YEAR TERM'
must_contain 'DUMP'

must_not_contain 'error: unknown command; use help'
must_not_contain 'error: target is not stopped at a breakpoint'
must_not_contain 'mcause=0x0000000000000002'
must_not_contain 'trap: illegal'

printf 'tutorial transcript validation passed: %s\n' "$transcript"
