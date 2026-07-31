    .section .text
    .globl _start
_start:
    la      t0, inputs
    la      t1, output

    csrw    fflags, zero
    flw     ft0, 0(t0)
    fcvt.w.s t3, ft0
    sw      t3, 0(t1)
    csrr    t2, fflags
    sw      t2, 8(t1)

    csrw    fflags, zero
    flw     ft0, 4(t0)
    fcvt.w.s t3, ft0, rdn
    sw      t3, 16(t1)
    csrr    t2, fflags
    sw      t2, 24(t1)

    csrw    fflags, zero
    flw     ft0, 8(t0)
    fcvt.wu.s t3, ft0
    sw      t3, 32(t1)
    csrr    t2, fflags
    sw      t2, 40(t1)

    csrw    fflags, zero
    flw     ft0, 12(t0)
    fcvt.wu.s t3, ft0
    sw      t3, 48(t1)
    csrr    t2, fflags
    sw      t2, 56(t1)

    csrw    fflags, zero
    fld     ft0, 16(t0)
    fcvt.w.d t3, ft0
    sw      t3, 64(t1)
    csrr    t2, fflags
    sw      t2, 72(t1)

    csrw    fflags, zero
    li      t3, -123
    fcvt.s.w ft1, t3
    fsw     ft1, 80(t1)
    csrr    t2, fflags
    sw      t2, 88(t1)

    csrw    fflags, zero
    li      t3, -1
    fcvt.s.wu ft1, t3
    fsw     ft1, 96(t1)
    csrr    t2, fflags
    sw      t2, 104(t1)

    csrw    fflags, zero
    li      t3, 0x80000000
    fcvt.d.w ft1, t3
    fsd     ft1, 112(t1)
    csrr    t2, fflags
    sd      t2, 120(t1)

    csrw    fflags, zero
    flw     ft0, 0(t0)
    fcvt.l.s t3, ft0
    sd      t3, 128(t1)
    csrr    t2, fflags
    sd      t2, 136(t1)

    csrw    fflags, zero
    flw     ft0, 12(t0)
    fcvt.lu.s t3, ft0
    sd      t3, 144(t1)
    csrr    t2, fflags
    sd      t2, 152(t1)

    csrw    fflags, zero
    fld     ft0, 16(t0)
    fcvt.l.d t3, ft0
    sd      t3, 160(t1)
    csrr    t2, fflags
    sd      t2, 168(t1)

    csrw    fflags, zero
    li      t3, 0x8000000000000000
    fcvt.d.l ft1, t3
    fsd     ft1, 176(t1)
    csrr    t2, fflags
    sd      t2, 184(t1)

    csrw    fflags, zero
    li      t3, -1
    fcvt.d.lu ft1, t3
    fsd     ft1, 192(t1)
    csrr    t2, fflags
    sd      t2, 200(t1)

    li      a0, 1
    la      a1, output
    li      a2, 208
    li      a7, 64
    ecall

    li      a0, 0
    li      a7, 93
    ecall

    .section .rodata
    .balign 8
inputs:
    .word   0x3fe00000              # 1.75
    .word   0xbfe00000              # -1.75
    .word   0x40600000              # 3.5
    .word   0xbf800000              # -1.0
    .dword  0x7ff0000000000000      # +infinity

    .section .bss
    .balign 8
output:
    .space 208
