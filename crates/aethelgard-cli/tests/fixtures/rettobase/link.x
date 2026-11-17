/* AET-WO-4400e3 RETTOBASE fixture: modelled on tests/fixtures/forge-log/link.x */
MEMORY
{
  /* FLASH_BANK1_BASE, up to FLASH_BANK2_BASE: bank 1. */
  FLASH : ORIGIN = 0x08000000, LENGTH = 0x100000
  /* D1_DTCMRAM_BASE, of the 128 KB its comment states. */
  RAM   : ORIGIN = 0x20000000, LENGTH = 128K
}

ENTRY(Reset);

SECTIONS
{
  .vector_table ORIGIN(FLASH) :
  {
    LONG(ORIGIN(RAM) + LENGTH(RAM));   /* initial SP */
    KEEP(*(.vector_table.exceptions)); /* reset and the 14 core exceptions */
    KEEP(*(.vector_table.interrupts)); /* the STM32H753's 150 external interrupts */
  } > FLASH

  .text : { *(.text .text.*); } > FLASH
  .rodata : { *(.rodata .rodata.*); . = ALIGN(4); } > FLASH

  /* The record: first in RAM and never initialised by the reset handler, so a reader sees the
     program's writes, not the reset value. */
  .aethelgard_boot (NOLOAD) : { KEEP(*(.aethelgard_boot)); } > RAM

  .data : AT(LOADADDR(.rodata) + SIZEOF(.rodata))
  {
    _sdata = .; *(.data .data.*); . = ALIGN(4); _edata = .;
  } > RAM
  _sidata = LOADADDR(.data);

  .bss (NOLOAD) : { . = ALIGN(4); _sbss = .; *(.bss .bss.*); . = ALIGN(4); _ebss = .; } > RAM

  /DISCARD/ : { *(.ARM.exidx .ARM.exidx.*); }
}
