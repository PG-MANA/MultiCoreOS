

.equ MULTIBOOT_CHECK_MAGIC, 0x36d76289 /* Multiboot2 magic code */
.equ STACK_SIZE, 0x8000
.equ IO_MAP_SIZE,0xffff

.global boot_entry, main_code_segment_descriptor, gdtr0, pml4
.extern boot_main

.section .text
.align 4

.code32
boot_entry:
  mov   $(stack + STACK_SIZE), %esp

  push  $0
  popfd
  push  $0                          /* for 64bit pop */
  push  %ebx                        /* Multiboot Informationのアドレスの保存 */

  cmp   $MULTIBOOT_CHECK_MAGIC, %eax
  jne   fin

  /* 割り込み禁止 */
  mov  $0xff, %al
  out  %al, $0x21
  nop
  out  %al, $0xa1
  cli

  /* TSSセグメント記述子にアドレスを記入 */
  mov   $tss, %eax
  mov   $tss_descriptor_address, %ebp
  mov   %ax, 2(%ebp)
  shr   $16, %eax
  mov   %al, 4(%ebp)
  mov   %ah, 7(%ebp)
  /* ロングモード対応か確認 */
  pushfd
  pop   %eax
  mov   %eax, %ecx
  xor   $(1 << 21), %eax
  push  %eax
  popfd
  pushfd
  pop   %eax
  push  %ecx
  /* EFLAGS復元 */
  popfd
  xor   %ecx, %eax
  jz    fin
  mov   $0x80000000, %eax
  /* 拡張CPUIDは有効か? */
  cpuid
  cmp   $0x80000001, %eax
  jb    fin
  mov   $0x80000001, %eax
  cpuid
  test  $(1 << 29), %edx
  /* Long Mode Enable Bit */
  jz    fin

  /* Paging */
  /* 2MiBページングを有効化 */
  /* PML4->PDP->PD */
  /* 先頭4GiBを仮想アドレス = 物理アドレスでマップ */
  xor   %ecx,  %ecx
pde_setup:
  mov   $0x200000, %eax
  mul   %ecx
  or    $0b10000011, %eax
  mov   %eax, pd(,%ecx, 8)
  inc   %ecx
  cmp   $2048, %ecx
  jne   pde_setup

  xor   %ecx, %ecx
pdpte_setup:
  mov   $4096, %eax
  mul   %ecx
  add   $pd, %eax
  or    $0b11, %eax
  mov   %eax, pdpt(,%ecx, 8)
  inc   %ecx
  cmp   $4, %ecx
  jne   pdpte_setup

/* pml4_setup: */
  mov   $pdpt, %eax
  or    $0b11, %eax
  mov   %eax, (pml4)

/* setup_64: */
  /* CR3にPML4のアドレスをセットし、
     CR4のPAEフラグとMSR(0xc0000080)のLMEとNXEをセット、
     最後にページングを有効化しGDTをセット */
  mov   $pml4, %eax
  mov   %eax, %cr3
  mov   %cr4, %eax
  or    $(1 << 5), %eax
  mov   %eax, %cr4
  mov   $0xc0000080, %ecx
  rdmsr
  or    $(1 << 8 | 1 << 11), %eax
  wrmsr
  mov   %cr0, %eax
  or    $(1 << 31 | 1), %eax
  lgdt  gdtr0
  mov   %eax, %cr0
  ljmp $main_code_segment_descriptor, $jump_to_rust


fin:
  cli
  hlt
  jmp fin

.code64

jump_to_rust:
  xor   %ax, %ax
  mov   %ax, %es
  mov   %ax, %ss
  mov   %ax, %ds
  mov   %ax, %fs
  mov   %ax, %gs
  mov   $tss_descriptor, %ax
  ltr   %ax

  pop   %rdi
  jmp   boot_main
  
.section .data

.comm stack, STACK_SIZE, 0x1000

/* PAGE DIRECTPRY (8byte * 512) * 4 */
.comm pd, 0x4000, 0x1000

/* PAGE DIRECTPRY POINTER TABLE (8byte * 512[4 entries are used]) */
.comm pdpt, 0x1000, 0x1000

/* PML4 (8byte * 512[1 entry is used]) */
.comm pml4, 0x1000, 0x1000

tss:
  .rept     25
    .long    0
  .endr
  .word     0
  .word     tss_io_map - tss
tss_io_map:
  .rept     IO_MAP_SIZE / 8
    .byte   0xff
  .endr
  .byte     0xff
tss_end:

.align 8

gdt:
    .quad    0

.equ  main_code_segment_descriptor, . - gdt
    .quad    (1 << 41) | (1 << 43) | (1 << 44) | (1 << 47) | (1 << 53)

.equ user_code_segment_descriptor, . - gdt
    .quad    (1 << 41) | (1 << 43) | (1 << 44) | (3 << 45) | (1 << 47) | (1 << 53)

.equ user_data_segment_descriptor, . - gdt
    .quad    (1 << 41) | (1 << 44) | (3 << 45) | (1 << 47)| (1 << 53)

tss_descriptor_address:
.equ  tss_descriptor, tss_descriptor_address - gdt
    .word    (tss_end - tss) & 0xffff               /* Limit(Low) */
    .word    0                                      /* Base(Low) */
    .byte    0                                      /* Base(middle) */
    .byte    0b10001001                             /* 64bit TSS + DPL:0 + P:1 */
    .byte    ((tss_end - tss) & 0xff0000) >> 0x10   /* Limit(High)+Granularity */
    .byte    0                                      /* Base(Middle high) */
    .long    0                                      /* Base(High) */
    .word    0                                      /* Reserved */
    .word    0                                      /* Reserved */

gdtr0:
  .word    . - gdt - 1                  /* The byte size of descriptors */
  .quad    gdt

