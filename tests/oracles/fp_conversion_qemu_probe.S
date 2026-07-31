    .section .text
    .globl _start
_start:
    la      t0, inputs
    la      t1, output

    csrw    fflags, zero
    fld     ft0, 0(t0)
    fcvt.s.d ft1, ft0
    fsw     ft1, 0(t1)
    csrr    t2, fflags
    sw      t2, 8(t1)

    csrw    fflags, zero
    fld     ft0, 0(t0)
    fcvt.s.d ft1, ft0, rup
    fsw     ft1, 16(t1)
    csrr    t2, fflags
    sw      t2, 24(t1)

    csrw    fflags, zero
    flw     ft0, 8(t0)
    fcvt.d.s ft1, ft0
    fsd     ft1, 32(t1)
    csrr    t2, fflags
    sd      t2, 40(t1)

    li      a0, 1
    la      a1, output
    li      a2, 48
    li      a7, 64
    ecall

    li      a0, 0
    li      a7, 93
    ecall

    .section .rodata
    .balign 8
inputs:
    # binary64 1 + 2^-24, exactly halfway between binary32 values
    .dword  0x3ff0000010000000
    # binary32 1.5
    .word   0x3fc00000

    .section .bss
    .balign 8
output:
    .space 48
