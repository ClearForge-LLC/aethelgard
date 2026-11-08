MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 0x100000
  RAM   : ORIGIN = 0x20000000, LENGTH = 128K
}
ENTRY(Reset);
SECTIONS
{
  .vector_table ORIGIN(FLASH) :
  {
    LONG(ORIGIN(RAM) + LENGTH(RAM));
    KEEP(*(.vector_table.exceptions));
  } > FLASH
  .text : { *(.text .text.*); } > FLASH
  .rodata : { *(.rodata .rodata.*); . = ALIGN(4); } > FLASH
  /* Result block at 0x20000000, control block at 0x20000100; NOLOAD, never zeroed by reset. */
  .s3_result 0x20000000 (NOLOAD) : { KEEP(*(.s3_result)); } > RAM
  .s3_ctl    0x20000100 (NOLOAD) : { KEEP(*(.s3_ctl)); } > RAM
  /DISCARD/ : { *(.ARM.exidx .ARM.exidx.*); }
}
