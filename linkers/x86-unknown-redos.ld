ENTRY(main)
OUTPUT_FORMAT(elf32-i386)

/* The start address must match redos kernel */
OFFSET = 0x100000;

SECTIONS {
    . = OFFSET;

    . += SIZEOF_HEADERS;
    . = ALIGN(4096);

    .text : AT(ADDR(.text) - OFFSET) {
        __text_start = .;
        *(.text*)
        . = ALIGN(4096);
        __text_end = .;
    }

    .rodata : AT(ADDR(.rodata) - OFFSET) {
        __rodata_start = .;
        *(.rodata*)
        . = ALIGN(4096);
        __rodata_end = .;
    }

    .data : AT(ADDR(.data) - OFFSET) {
        __data_start = .;
        *(.data*)
        . = ALIGN(4096);
        __data_end = .;
        __bss_start = .;
        *(.bss*)
        . = ALIGN(4096);
        __bss_end = .;
    }

    .tdata : AT(ADDR(.tdata) - OFFSET) {
        __tdata_start = .;
        *(.tdata*)
        . = ALIGN(4096);
        __tdata_end = .;
        __tbss_start = .;
        *(.tbss*)
        . += 8;
        . = ALIGN(4096);
        __tbss_end = .;
    }

    __end = .;

    /DISCARD/ : {
        *(.comment*)
        *(.eh_frame*)
        *(.gcc_except_table*)
        *(.note*)
        *(.rel.eh_frame*)
    }
}
