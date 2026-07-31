    .section .text
    .globl _start
_start:
    la      t0, inputs
    la      t1, output

    csrw    fflags, zero
    flw     ft0, 0(t0)
    flw     ft1, 4(t0)
    fadd.s  ft2, ft0, ft1
    fsw     ft2, 0(t1)
    csrr    t2, fflags
    sw      t2, 8(t1)

    csrw    fflags, zero
    flw     ft0, 8(t0)
    flw     ft1, 12(t0)
    fadd.s  ft2, ft0, ft1
    fsw     ft2, 16(t1)
    csrr    t2, fflags
    sw      t2, 24(t1)

    csrw    fflags, zero
    flw     ft0, 16(t0)
    flw     ft1, 20(t0)
    fadd.s  ft2, ft0, ft1
    fsw     ft2, 32(t1)
    csrr    t2, fflags
    sw      t2, 40(t1)

    csrw    fflags, zero
    flw     ft0, 24(t0)
    flw     ft1, 28(t0)
    fadd.s  ft2, ft0, ft1
    fsw     ft2, 48(t1)
    csrr    t2, fflags
    sw      t2, 56(t1)

    csrw    fflags, zero
    fld     ft0, 32(t0)
    fld     ft1, 40(t0)
    fadd.d  ft2, ft0, ft1
    fsd     ft2, 64(t1)
    csrr    t2, fflags
    sd      t2, 72(t1)

    csrw    fflags, zero
    fld     ft0, 48(t0)
    fld     ft1, 56(t0)
    fadd.d  ft2, ft0, ft1
    fsd     ft2, 80(t1)
    csrr    t2, fflags
    sd      t2, 88(t1)

    csrw    fflags, zero
    fld     ft0, 64(t0)
    fld     ft1, 72(t0)
    fadd.d  ft2, ft0, ft1
    fsd     ft2, 96(t1)
    csrr    t2, fflags
    sd      t2, 104(t1)

    csrw    fflags, zero
    fld     ft0, 80(t0)
    fld     ft1, 88(t0)
    fadd.d  ft2, ft0, ft1
    fsd     ft2, 112(t1)
    csrr    t2, fflags
    sd      t2, 120(t1)

    li      a0, 1
    la      a1, output
    li      a2, 128
    li      a7, 64
    ecall

    li      a0, 0
    li      a7, 93
    ecall

    .section .rodata
    .balign 8
inputs:
    # binary32: 1.5+2.25, max+max, min-subnormal+min-subnormal, +inf + -inf
    .word   0x3fc00000, 0x40100000
    .word   0x7f7fffff, 0x7f7fffff
    .word   0x00000001, 0x00000001
    .word   0x7f800000, 0xff800000
    # binary64: 1.5+2.25, max+max, min-subnormal+min-subnormal, +inf + -inf
    .dword  0x3ff8000000000000, 0x4002000000000000
    .dword  0x7fefffffffffffff, 0x7fefffffffffffff
    .dword  0x0000000000000001, 0x0000000000000001
    .dword  0x7ff0000000000000, 0xfff0000000000000

    .section .bss
    .balign 8
output:
    .space 128
