/* AET-WO-2100 spike: STM32H753 memory, from stm32h753xx.h:
   FLASH_BANK1_BASE 0x08000000 (1 MB bank), D1_DTCMRAM_BASE 0x20000000 (128 KB). */
MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 1024K
  RAM   : ORIGIN = 0x20000000, LENGTH = 128K
}

/* The observable word (S3): first in RAM, never initialised by the runtime.
   INSERT BEFORE .data makes it the first RAM output section. */
SECTIONS
{
  .aethelgard_boot (NOLOAD) : { KEEP(*(.aethelgard_boot)); } > RAM
} INSERT BEFORE .data;
