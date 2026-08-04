#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

image=target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor
asm_code="target/payloads/minibasic-payload-asm.bin"
asm_data="target/payloads/minibasic-payload-asm-data.bin"
asm_listing="target/payloads/minibasic-payload-asm.objdump.txt"
test -s "$asm_code"
test -s "$asm_data"
test -s "$asm_listing"
divide_offset="$(awk '$2 == "<integrated_reduce_div>:" { print $1; exit }' "$asm_listing")"
test -n "$divide_offset"
divide_address="$(printf '0x%x' "$((0x81000100 + 16#$divide_offset))")"
pause="${TUTORIAL_GUEST_PAUSE:-1}"

send() {
    printf '%s\n' "$1"
    sleep "$pause"
}

# Les caractères envoyés dans le pipe QEMU ne sont pas un écho de terminal.
# Ces annotations sont donc écrites sur stderr, qui reste visible dans le cast,
# et ne sont jamais interprétées comme des commandes par le guest.
note() {
    printf '\r\n=== %s ===\r\n' "$1" >&2
    sleep "$pause"
}

send_fast() {
    printf '%s\n' "$1"
    sleep 0.002
}

preview_asm_source() {
    while IFS= read -r line; do
        printf 'asm-source> %s\r\n' "$line" >&2
        sleep 0.002
    done < examples/minibasic-asm/payload-repl.rv
}

stream_payload_binary() {
    local command="$1"
    local base="$2"
    local path="$3"
    local offset=0
    while IFS= read -r hex; do
        [[ -z "$hex" ]] && continue
        send_fast "$command $(printf '0x%x' "$((base + offset))") $hex"
        offset=$((offset + ${#hex} / 2))
    done < <(od -An -v -tx1 -w32 "$path" | awk '{gsub(/[[:space:]]/, ""); if (length) print}')
}

{
    sleep "$pause"

    note 'MONITEUR GUEST — M-mode, cible U-mode et services console'
    # Sections 3 and 4: help, registers, memory, edit, undo and data.
    send 'help'
    send 'regs'
    send 'memory 0x80000000 16'
    send 'edit 0x80010040 deadbeef'
    send 'memory 0x80010040 4'
    send 'undo'
    send 'data 0x82000060 .float 0x3f800000'
    send 'memory 0x82000060 4'
    send 'data 0x82000060 .binary128 000102030405060708090a0b0c0d0e0f'

    # Section 3: first addi cycle.
    send 'assemble 0x81000100 addi x1,x0,1'
    send 'step'
    send 'regs'

    # Multi-line source, symbols and disassembly.
    send 'assemble-program 0x81000a30'
    send '_start:'
    send 'addi x1,x0,1'
    send 'beq x1,x1,next'
    send 'addi x1,x0,99'
    send 'next:'
    send 'addi x1,x1,2'
    send 'ebreak'
    send 'end'
    send 'symbols'
    send 'disasm _start 5'
    send 'run-at 0x81000a30'
    send 'regs'

    # Source diagnostics and correction workflow.
    send 'assemble-program 0x81001000'
    send 'addi x1,x0,7'
    send 'not-an-instruction x1'
    send 'end'
    send 'assemble-program 0x81001000'
    send 'addi x1,x0,7'
    send 'addi x1,x1,1'
    send 'end'
    send 'source'
    send 'source replace 2 "addi x1,x1,5"'
    send 'assemble-source'

    note 'ASM MINIBASIC-RV — aperçu source assembleur, non exécuté par l’hôte (500 lignes/s)'
    preview_asm_source
    note 'ASM MINIBASIC-RV — chargement binaire explicite via les commandes du moniteur'
    stream_payload_binary 'payload-load' 0x81000100 "$asm_code"
    stream_payload_binary 'payload-load-data' 0x82000000 "$asm_data"
    send 'info payload'
    # Preuve que l'image chargée a laissé ses propres métadonnées en RAM cible.
    send 'memory 0x82000100 33'
    send "break $divide_address"
    send 'run-at 0x81000100'

    # Le payload explicitement transféré est maintenant réellement piloté :
    # cette ligne atteint son propre integrated_reduce_div/fdiv.d breakpoint.
    send '10 I=1'
    send '20 X=I/3'
    send '30 PRINT X'
    send '40 END'
    send 'RUN'
    send 'regs'
    send "disasm $divide_address 6"
    send 'step'
    send 'regs'
    send 'delete 1'

    # Floating point motifs and exact register views.
    send 'assemble 0x81002000 ebreak'
    send 'run-at 0x81002000'
    send 'setf f1 0xffffffff3f800000'
    send 'setf f2 0xffffffff40000000'
    send 'assemble 0x81002000 fadd.s f3,f1,f2'
    send 'step'
    send 'regs'
    send 'assemble 0x81002004 ebreak'
    send 'run-at 0x81002004'
    send 'setf f4 0x3ff0000000000000'
    send 'setf f5 0x4000000000000000'
    send 'assemble 0x81002004 fadd.d f6,f4,f5'
    send 'step'
    send 'regs'

    # Continue, snapshots and software watchpoints.
    send 'set x1 0x7'
    send 'snapshot save'
    send 'set x1 0x99'
    send 'snapshot restore'
    send 'regs'
    send 'snapshot info'
    send 'snapshot manifest'
    send 'set x4 0x0000000082000060'
    send 'watch 0x82000060 8'
    send 'info watch'
    send 'delete watch 1'

    note 'ASM MINIBASIC-RV — payload assembleur chargé et exécuté depuis le workspace'
    note 'RUST MINIBASIC-RV — référence legacy résidente, non utilisée dans ce parcours'
    note 'ASM MINIBASIC-RV — sections progressives du tutoriel'
    # Sections 4.1–4.5: direct mode, storage, FOR, TRACE, INPUT and D.
    send 'basic'
    send 'PRINT 2+3*4'
    send 'PRINT (2+3)*4'
    send 'PRINT 22/7'
    send '30 PRINT "DONE"'
    send '40 END'
    send '10 A=2'
    send '20 PRINT A*A'
    send 'LIST'
    send 'RUN'
    send 'basic'
    send '10 FOR I=1 TO 3'
    send '20 PRINT I,I*I'
    send '30 NEXT I'
    send '40 END'
    send 'LIST'
    send 'TRACE ON'
    send 'RUN'
    send 'basic'
    send '10 INPUT N'
    send '20 IF N<0 THEN 50'
    send '30 PRINT N*N'
    send '40 GOTO 10'
    send '50 END'
    send 'RUN'
    send '3'
    send '4'
    send '-1'

    note 'ASM MINIBASIC-RV — jeu final HAMMURABI-RV'
    # Section 4.7: complete tutorial game, copied from the documented listing.
    send 'basic'
    awk '/^### 4.7/{found=1} found && /^```basic/{program=1; next} program && /^```/{exit} program{print}' \
        docs/TUTORIAL-GUEST.md |
        while IFS= read -r line; do
            send "$line"
        done
    send 'LIST'
    send 'DUMP'
    send 'TRACE ON'
    send 'RUN'
    for value in 0 20 190 0 20 190 0 20 190 0 20 190 0 20 190; do
        send "$value"
    done
    send 'regs'
    send 'snapshot save'
    send 'snapshot info'
    send 'q'
} | timeout "${TUTORIAL_GUEST_TIMEOUT:-600}s" qemu-system-riscv64 \
    -M virt -m 64M -bios none -kernel "$image" -nographic
