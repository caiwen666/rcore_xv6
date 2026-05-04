OUTPUT_ARCH( "riscv" )
ENTRY( _entry )

SECTIONS
{
    /*
     * QEMU 约定初始时从 0x80000000 这个位置开始执行
     */
    . = 0x80000000;

    .text : {
        *(.text.entry .text.entry.*)
        *(.text .text.*)
    }

    . = ALIGN(4K);
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    . = ALIGN(4K);
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    . = ALIGN(4K);
    .bss : {
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }
}