
.global ap_entry, ap_entry_end, ap_os_stack_address

.extern main_code_segment_descriptor, gdtr0, pml4
.extern ap_boot_main

.section .data

.code16
ap_entry:
    /* 16bitリアルモードではCS・DSレジスタはアドレス計算時に4bitした値が
       オフセットと合算され実際のアドレスが求められる。"cs:ip(address = cs * 16 + ip)" */
    cli
    /* CSレジスタからap_entryのアドレスを計算する */
    mov     %cs, %ax
    mov     %ax, %ds    /* 全てのメモリデータアクセスにはDSレジスタが使用される */
    xor     %ebx, %ebx  /* EBX = 0 */
    mov     %ax, %bx
    shl     $4, %ebx    /* EBX <<=4 ( EBX *= 16 ) */

    /* ljmplとGDTのベースアドレスを調整 */
    add     %ebx, ljmpl_32_address - ap_entry
    add     %ebx, gdtr_32bit - ap_entry + 2

    lgdt    (gdtr_32bit - ap_entry)

    mov     %cr0, %eax
    and     $0x7fffffff, %eax   /* ページング無効 */
    or      $0x00000001, %eax   /* 32bitプロテクトモード */
    mov     %eax, %cr0

    /* Long JMP */
    .byte 0x66, 0xea    /* オペコードと32bitアドレスプレフィックス */
ljmpl_32_address:
    .long (ap_init_long_mode - ap_entry)    /* オフセット */
    .word gdt_32bit_code_segment_descriptor /* コードセグメント */


.code32
ap_init_long_mode:
    mov     $gdt_32bit_data_segment_descriptor, %ax
    mov     %ax, %ds

    /* ljmplのベースアドレスを調整 */
    mov    $(ljmpl_64_address - ap_entry), %eax
    add     %ebx, (%ebx, %eax)

    mov     $pml4, %eax
    mov     %eax, %cr3
    mov     %cr4, %eax
    or      $(1 << 5), %eax
    mov     %eax, %cr4                  /* Set PAE flag */
    mov     $0xc0000080, %ecx
    rdmsr                               /* Model-specific register */
    or      $(1 << 8 | 1 << 11), %eax
    wrmsr                               /* Set LME and NXE flags */
    mov     %cr0, %eax
    or      $(1 << 31 | 1), %eax        /* Set PG flag */
    lgdt    gdtr0
    mov     %eax, %cr0

    /* Long JMP */
    .byte   0xea                        /* オペコード */
ljmpl_64_address:
    .long (ap_init_x86_64 - ap_entry)   /* オフセット */
    .word main_code_segment_descriptor  /* コードセグメント */


.code64
ap_init_x86_64:
    xor     %ax, %ax
    mov     %ax, %es
    mov     %ax, %ss
    mov     %ax, %ds
    mov     %ax, %fs
    mov     %ax, %gs
    /* スタックをセット */
    mov     $(ap_os_stack_address - ap_entry), %eax
    add     %ebx, %eax      /* EBXがベースアドレスを保持してる */
    mov     (%eax), %rsp
    lea     ap_boot_main, %rax
    jmp    *%rax            /* "*"は絶対ジャンプ */


.align  16

gdt_32bit:
    .quad   0
.equ gdt_32bit_code_segment_descriptor, . - gdt_32bit
    .word   0xffff, 0x0000, 0x9b00, 0x00cf
.equ gdt_32bit_data_segment_descriptor, . - gdt_32bit
    .word   0xffff, 0x0000, 0x9200, 0x00cf
    .word   0
gdtr_32bit:
    .word  . - gdt_32bit - 1
    .long  gdt_32bit - ap_entry

.align 8

ap_os_stack_address:
    .quad   0

ap_entry_end:
 
