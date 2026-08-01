ENTRY(_start)

MEMORY
{
    RAM (rwx) : ORIGIN = 0x80000000, LENGTH = 64M
}

SECTIONS
{
    .text : ALIGN(4)
    {
        KEEP(*(.text.init))
        *(.text .text.*)
    } > RAM

    .rodata : ALIGN(8)
    {
        *(.rodata .rodata.*)
    } > RAM

    .data : ALIGN(8)
    {
        *(.data .data.*)
    } > RAM

    .bss : ALIGN(8)
    {
        *(.bss .bss.*)
        *(COMMON)
    } > RAM

    .stack (NOLOAD) : ALIGN(16)
    {
        _stack_bottom = .;
        . += 64K;
        _stack_top = .;
    } > RAM

    /* Leave a large unmapped-in-the-image gap after the monitor so target
       code is visibly separate from the monitor's fixed image. */
    .target_workspace 0x81000000 (NOLOAD) : ALIGN(16)
    {
        _target_workspace_start = .;
        . += 64K;
        _target_workspace_end = .;
    } > RAM

    /* Target variables live in a distinct, farther region. The monitor's
       data directives are restricted to this range. */
    .target_data 0x82000000 (NOLOAD) : ALIGN(16)
    {
        _target_data_start = .;
        . += 1M;
        _target_data_end = .;
    } > RAM

    /DISCARD/ :
    {
        *(.eh_frame)
        *(.comment)
        *(.riscv.attributes)
    }
}
