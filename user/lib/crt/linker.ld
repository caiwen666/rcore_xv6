OUTPUT_ARCH(riscv)
ENTRY(_start)

BASE_ADDRESS = 0x1000;

SECTIONS
{
    . = BASE_ADDRESS;

    .text : {
        *(.text.entry)
        *(.text .text.*)
    }

    . = ALIGN(16);
    etext = .;

    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    . = ALIGN(16);
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    . = ALIGN(16);
    edata = .;
    __tls_base = .;
    .tdata : {
        *(.tdata .tdata.*)
    }
    __tls_data_end = .;

    . = ALIGN(16);
    .tbss : {
        *(.tbss .tbss.*)
    }
    __tls_bss_end = .;

    .bss : {
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }

    end = .;
    _end = .;

    /DISCARD/ : {
        *(.eh_frame)
        *(.comment)
        *(.note*)
        *(.debug*)
    }
}
