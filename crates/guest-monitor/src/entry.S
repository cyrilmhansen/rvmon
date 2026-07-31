.section .text.init
.option norvc
.globl _start
.type _start, @function
_start:
    la sp, _stack_top
    call rust_main
1:
    wfi
    j 1b
.size _start, .-_start

.text
.globl install_trap
.type install_trap, @function
install_trap:
    csrw mtvec, a0
    csrw mscratch, a1
    # V1 exposes the low physical window through PMP to the U-mode target.
    # TOR [0, 0x80020000) covers the QEMU virt UART and the 128 KiB target RAM.
    li t1, 0x20008000
    csrw pmpaddr0, t1
    li t1, 0x0f
    csrw pmpcfg0, t1
    csrr t0, mstatus
    li t1, 0x6000
    or t0, t0, t1
    csrw mstatus, t0
    ret
.size install_trap, .-install_trap

.globl enter_user
.type enter_user, @function
enter_user:
    csrw mepc, a0
    csrr t0, mstatus
    li t1, 0x1800
    not t1, t1
    and t0, t0, t1
    li t1, 0x80
    or t0, t0, t1
    csrw mstatus, t0
    mv sp, a1
    mret
.size enter_user, .-enter_user

.globl resume_user
.type resume_user, @function
resume_user:
    # a0 is the saved TargetContext. Keep it in t6 while restoring all
    # target registers; t6 itself is restored last from x31.
    mv t6, a0

    fld f0, 256(t6)
    fld f1, 264(t6)
    fld f2, 272(t6)
    fld f3, 280(t6)
    fld f4, 288(t6)
    fld f5, 296(t6)
    fld f6, 304(t6)
    fld f7, 312(t6)
    fld f8, 320(t6)
    fld f9, 328(t6)
    fld f10, 336(t6)
    fld f11, 344(t6)
    fld f12, 352(t6)
    fld f13, 360(t6)
    fld f14, 368(t6)
    fld f15, 376(t6)
    fld f16, 384(t6)
    fld f17, 392(t6)
    fld f18, 400(t6)
    fld f19, 408(t6)
    fld f20, 416(t6)
    fld f21, 424(t6)
    fld f22, 432(t6)
    fld f23, 440(t6)
    fld f24, 448(t6)
    fld f25, 456(t6)
    fld f26, 464(t6)
    fld f27, 472(t6)
    fld f28, 480(t6)
    fld f29, 488(t6)
    fld f30, 496(t6)
    fld f31, 504(t6)

    ld ra, 8(t6)
    ld sp, 16(t6)
    ld gp, 24(t6)
    ld tp, 32(t6)
    ld t0, 40(t6)
    ld t1, 48(t6)
    ld t2, 56(t6)
    ld s0, 64(t6)
    ld s1, 72(t6)
    ld a0, 80(t6)
    ld a1, 88(t6)
    ld a2, 96(t6)
    ld a3, 104(t6)
    ld a4, 112(t6)
    ld a5, 120(t6)
    ld a6, 128(t6)
    ld a7, 136(t6)
    ld s2, 144(t6)
    ld s3, 152(t6)
    ld s4, 160(t6)
    ld s5, 168(t6)
    ld s6, 176(t6)
    ld s7, 184(t6)
    ld s8, 192(t6)
    ld s9, 200(t6)
    ld s10, 208(t6)
    ld s11, 216(t6)
    ld t3, 224(t6)
    ld t4, 232(t6)
    ld t5, 240(t6)

    ld t0, 512(t6)
    csrw mepc, t0
    ld t0, 520(t6)
    csrw fcsr, t0
    ld t0, 528(t6)
    csrw mstatus, t0
    csrw mscratch, t6
    ld t6, 248(t6)
    mret
.size resume_user, .-resume_user

.align 2
.globl target_entry
.type target_entry, @function
target_entry:
    addi x1, x0, 1
    ebreak
    addi x1, x1, 1
    beq x1, x1, 8
    ebreak
    ebreak
target_halt:
    wfi
    j target_halt
.size target_entry, .-target_entry

.align 2
.globl trap_entry
.type trap_entry, @function
trap_entry:
    csrrw t0, mscratch, t0

    sd ra, 8(t0)
    sd sp, 16(t0)
    sd gp, 24(t0)
    sd tp, 32(t0)
    sd t1, 48(t0)
    sd t2, 56(t0)
    sd s0, 64(t0)
    sd s1, 72(t0)
    sd a0, 80(t0)
    sd a1, 88(t0)
    sd a2, 96(t0)
    sd a3, 104(t0)
    sd a4, 112(t0)
    sd a5, 120(t0)
    sd a6, 128(t0)
    sd a7, 136(t0)
    sd s2, 144(t0)
    sd s3, 152(t0)
    sd s4, 160(t0)
    sd s5, 168(t0)
    sd s6, 176(t0)
    sd s7, 184(t0)
    sd s8, 192(t0)
    sd s9, 200(t0)
    sd s10, 208(t0)
    sd s11, 216(t0)
    sd t3, 224(t0)
    sd t4, 232(t0)
    sd t5, 240(t0)
    sd t6, 248(t0)
    csrr t1, mscratch
    sd t1, 40(t0)

    fsd f0, 256(t0)
    fsd f1, 264(t0)
    fsd f2, 272(t0)
    fsd f3, 280(t0)
    fsd f4, 288(t0)
    fsd f5, 296(t0)
    fsd f6, 304(t0)
    fsd f7, 312(t0)
    fsd f8, 320(t0)
    fsd f9, 328(t0)
    fsd f10, 336(t0)
    fsd f11, 344(t0)
    fsd f12, 352(t0)
    fsd f13, 360(t0)
    fsd f14, 368(t0)
    fsd f15, 376(t0)
    fsd f16, 384(t0)
    fsd f17, 392(t0)
    fsd f18, 400(t0)
    fsd f19, 408(t0)
    fsd f20, 416(t0)
    fsd f21, 424(t0)
    fsd f22, 432(t0)
    fsd f23, 440(t0)
    fsd f24, 448(t0)
    fsd f25, 456(t0)
    fsd f26, 464(t0)
    fsd f27, 472(t0)
    fsd f28, 480(t0)
    fsd f29, 488(t0)
    fsd f30, 496(t0)
    fsd f31, 504(t0)

    csrr t1, mepc
    sd t1, 512(t0)
    sd t1, 536(t0)
    csrr t1, mstatus
    sd t1, 528(t0)
    csrr t1, mcause
    sd t1, 544(t0)
    csrr t1, mtval
    sd t1, 552(t0)
    csrr t1, fcsr
    sd t1, 520(t0)

    mv a0, t0
    call rust_trap
1:
    wfi
    j 1b
.size trap_entry, .-trap_entry
