/* AET-WO-2100 spike: STM32H753 memory, from stm32h753xx.h:
   FLASH_BANK1_BASE 0x08000000 (1 MB bank), D1_DTCMRAM_BASE 0x20000000 (128 KB). */
MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 1024K
  RAM   : ORIGIN = 0x20000000, LENGTH = 128K
}

ENTRY(Reset);

SECTIONS
{
  .vector_table ORIGIN(FLASH) :
  {
    LONG(ORIGIN(RAM) + LENGTH(RAM));   /* initial SP */
    KEEP(*(.vector_table.exceptions)); /* reset and the 14 core exceptions */
  } > FLASH

  .text : { *(.text .text.*); } > FLASH
  .rodata : { *(.rodata .rodata.*); } > FLASH

  /* The observable word (S3): first in RAM, never initialised by the reset
     handler, so its reset value is Renode's (0), not the program's. */
  .aethelgard_boot (NOLOAD) : { KEEP(*(.aethelgard_boot)); } > RAM

  .data : AT(LOADADDR(.rodata) + SIZEOF(.rodata))
  {
    _sdata = .; *(.data .data.*); . = ALIGN(4); _edata = .;
  } > RAM
  _sidata = LOADADDR(.data);

  .bss (NOLOAD) : { _sbss = .; *(.bss .bss.*); . = ALIGN(4); _ebss = .; } > RAM

  /DISCARD/ : { *(.ARM.exidx .ARM.exidx.*); }
}
