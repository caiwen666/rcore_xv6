OUTPUT_ARCH( "riscv" )
ENTRY( _entry )

SECTIONS
{
    /*
     * QEMU 约定初始时从 0x80000000 这个位置开始执行
     */
    . = 0x80000000;
    stext = .;
    .text : {
        *(.text.entry .text.entry.*)
        *(.text .text.*)
    }

    . = ALIGN(4K);
    etext = .;
    strampoline = .;
    .trampoline : {
        *(.trampoline)
    }
    
    . = ALIGN(4K);
    etrampoline = .;
    srodata = .;

    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    . = ALIGN(4K);
    erodata = .;
    sdata = .;

    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    . = ALIGN(4K);
    edata = .;
    sbss = .;

    .bss : {
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }

    . = ALIGN(4K);
    ebss = .;
    ekernel = .;
}