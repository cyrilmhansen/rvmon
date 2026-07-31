    .section .text
    .globl _start
_start:
    la      t2, output
    li      t0, 0x80000001
    fmv.w.x ft0, t0
    fmv.x.w t1, ft0
    sd      t1, 0(t2)

    li      t0, 0x1234567887654321
    fmv.w.x ft1, t0
    fsd     ft1, 8(t2)

    li      t0, 0x7ff8000000000042
    fmv.d.x ft0, t0
    fmv.x.d t1, ft0
    sd      t1, 16(t2)

    li      t0, 0x8000000000000000
    fmv.d.x ft1, t0
    fsd     ft1, 24(t2)

    li      a0, 1
    la      a1, output
    li      a2, 32
    li      a7, 64
    ecall
    li      a0, 0
    li      a7, 93
    ecall

    .section .bss
    .balign 8
output:
    .space 32
