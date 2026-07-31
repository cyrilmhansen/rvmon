ENTRY(_start)

MEMORY
{
    RAM (rwx) : ORIGIN = 0x80000000, LENGTH = 128K
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
        . += 16K;
        _stack_top = .;
    } > RAM

    .target_workspace (NOLOAD) : ALIGN(16)
    {
        _target_workspace_start = .;
        . += 16K;
        _target_workspace_end = .;
    } > RAM

    /DISCARD/ :
    {
        *(.eh_frame)
        *(.comment)
        *(.riscv.attributes)
    }
}
