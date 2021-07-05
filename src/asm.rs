global_asm!(include_str!("asm/boot_header.s"), options(att_syntax));
global_asm!(include_str!("asm/boot.s"), options(att_syntax));
global_asm!(include_str!("asm/ap_boot.s"), options(att_syntax));
