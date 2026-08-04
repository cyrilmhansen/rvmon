ENTRY(_start)

MEMORY
{
    RAM (rwx) : ORIGIN = 0x81000000, LENGTH = 64K
}

SECTIONS
{
    .text : ALIGN(4)
    {
        _payload_text_start = .;
        KEEP(*(.text.start))
        *(.text .text.*)
        _payload_text_end = .;
    } > RAM

    .rodata : ALIGN(8)
    {
        _payload_rodata_start = .;
        *(.rodata .rodata.*)
        _payload_rodata_end = .;
    } > RAM

    .data : ALIGN(8)
    {
        _payload_data_start = .;
        *(.data .data.*)
        *(.payload_data)
        _payload_data_end = .;
    } > RAM

    /* Keep the fixed BASIC state in the flat payload image.  A raw loader
       does not have an ELF zero-fill phase, so this output section is
       deliberately PROGBITS and contains its zero-initialized bytes. */
    .payload_state : ALIGN(8)
    {
        _payload_bss_start = .;
        BYTE(0)
        *(.bss .bss.*)
        *(COMMON)
        _payload_bss_end = .;
    } > RAM

    /DISCARD/ :
    {
        *(.eh_frame)
        *(.comment)
        *(.riscv.attributes)
    }
}
