.equ MULTIBOOT_HEADER_MAGIC,    0xe85250d6
.equ MULTIBOOT_HEADER_ARCH,     0
.equ MULTIBOOT_HEADER_LEN,      multiboot_end - multiboot_start
.equ MULTIBOOT_HEADER_CHECKSUM, -(MULTIBOOT_HEADER_MAGIC + MULTIBOOT_HEADER_ARCH + MULTIBOOT_HEADER_LEN)
.equ MULTIBOOT_HEADER_FLAG,     1
.equ MULTIBOOT_HEADER_TAG_TYPE_END,     0   /* End tag */
.equ MULTIBOOT_HEADER_TAG_TYPE_FB,      5   /* Frame Buffer setting tag */
.equ MULTIBOOT_HEADER_TAG_TYPE_ALIGN,   6   /* Alignment requirement tag */

.section .header.multiboot, "a" /* Alloc flag */

.align 8

multiboot_start:
  .long      MULTIBOOT_HEADER_MAGIC
  .long      MULTIBOOT_HEADER_ARCH
  .long      MULTIBOOT_HEADER_LEN
  .long      MULTIBOOT_HEADER_CHECKSUM

multiboot_tags_start:
  .word      MULTIBOOT_HEADER_TAG_TYPE_ALIGN
  .word      MULTIBOOT_HEADER_FLAG
  .long      8
  .word      MULTIBOOT_HEADER_TAG_TYPE_FB
  .word      MULTIBOOT_HEADER_FLAG
  .long      20
  .long      800                       /* width */
  .long      600                       /* height */
  .long      32                         /* depth */
  .align     8
  .word      MULTIBOOT_HEADER_TAG_TYPE_END
  .word      MULTIBOOT_HEADER_FLAG
  .long      8
multiboot_end:
 
