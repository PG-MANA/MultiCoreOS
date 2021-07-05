//! Application Processorの初期化用コード

use super::acpi::ApicIdList;
use super::acpi_pm_timer::AcpiPmTimer;
use super::local_apic::{get_apic_id, send_interrupt_command};
use super::memory::MemoryManager;
use super::MEMORY_MANAGER;
use core::sync::atomic::AtomicBool;

/// 各プロセッサが個別に持つ構造体
/// gsレジスタにベースアドレスが保存されます。
#[repr(C)]
pub struct PerCpuData {
    #[allow(dead_code)]
    self_pointer: usize,
    local_apic_id: u32,
}

/// APが起動したかどうかの確認用フラグ
static AP_BOOT_COMPLETE_FLAG: AtomicBool = AtomicBool::new(false);

pub fn init_ap(apic_id_list: ApicIdList, pm_timer: &AcpiPmTimer) {
    /* ap_boot.s */
    extern "C" {
        fn ap_entry();
        fn ap_entry_end();
        static mut ap_os_stack_address: u64;
    }
    let ap_entry_address = ap_entry as *const fn() as usize;
    let ap_entry_end_address = ap_entry_end as *const fn() as usize;

    let boot_code_address = unsafe {
        MEMORY_MANAGER
            .alloc_with_align(ap_entry_end_address - ap_entry_address, 0x1000)
            .unwrap()
    };
    assert!(
        boot_code_address <= 0xff000,
        "Address :{:#X}",
        boot_code_address
    );

    let vector = ((boot_code_address >> 12) & 0xff) as u8;
    /* 起動用のアセンブリコードをコピー */
    unsafe {
        core::ptr::copy_nonoverlapping(
            ap_entry_address as *const u8,
            boot_code_address as *mut u8,
            ap_entry_end_address - ap_entry_address,
        )
    };

    /* BSP用のPerCpuDataを作成し、local_apic_idをセット */
    let mut per_cpu_data = create_per_cpu_data(unsafe { &MEMORY_MANAGER });
    let bsp_apic_id = get_apic_id() as u32;
    per_cpu_data.local_apic_id = bsp_apic_id;

    let mut num_of_cpu = 1usize;
    'ap_init_loop: for apic_id in apic_id_list {
        if apic_id == bsp_apic_id {
            continue;
        }
        num_of_cpu += 1;
        if apic_id > 0xff {
            panic!("Please enable x2APIC");
        }

        let stack_size = 0x8000;
        let stack = unsafe { MEMORY_MANAGER.alloc_with_align(stack_size, 0x10).unwrap() };
        /* スタックのアドレスを起動するAPが取得できるようにメモする */
        unsafe {
            *(((&mut ap_os_stack_address as *mut _ as usize) - ap_entry_address + boot_code_address)
                as *mut u64) = (stack + stack_size) as u64
        };

        AP_BOOT_COMPLETE_FLAG.store(false, core::sync::atomic::Ordering::Relaxed);

        send_interrupt_command(apic_id, 0b101 /*INIT*/, 1, 1 /*Assert*/, 0);

        pm_timer.busy_wait_us(100);

        send_interrupt_command(apic_id, 0b101 /*INIT*/, 1, 0 /* De-Assert */, 0);

        pm_timer.busy_wait_ms(10);

        send_interrupt_command(apic_id, 0b110 /* Startup IPI*/, 0, 1, vector);

        pm_timer.busy_wait_us(200);

        send_interrupt_command(apic_id, 0b110 /* Startup IPI*/, 0, 1, vector);
        for _wait in 0..5000
        /* APの初期化完了まで5秒待つ */
        {
            if AP_BOOT_COMPLETE_FLAG.load(core::sync::atomic::Ordering::Relaxed) {
                continue 'ap_init_loop;
            }
            pm_timer.busy_wait_ms(1);
        }
        panic!("Cannot init CPU(APIC ID: {})", apic_id);
    }

    if num_of_cpu != 1 {
        println!("Found {} CPUs", num_of_cpu);
    }
}

#[no_mangle]
extern "C" fn ap_boot_main() -> ! {
    let mut per_cpu_data = create_per_cpu_data(unsafe { &MEMORY_MANAGER });
    per_cpu_data.local_apic_id = get_apic_id() as u32;
    drop(per_cpu_data);
    println!(
        "Hello! Local Apic id = {}",
        get_per_cpu_data().local_apic_id
    );
    AP_BOOT_COMPLETE_FLAG.store(true, core::sync::atomic::Ordering::Relaxed);
    loop {
        unsafe { asm!("hlt") };
    }
}

fn create_per_cpu_data(memory_manager: &MemoryManager) -> &'static mut PerCpuData {
    let address = memory_manager
        .alloc(core::mem::size_of::<PerCpuData>())
        .unwrap();
    let mut d = unsafe { &mut *(address as *mut PerCpuData) };
    d.self_pointer = address;
    let edx: u32 = (address >> 32) as u32;
    let eax: u32 = address as u32;
    unsafe { asm!("wrmsr", in("eax") eax, in("edx") edx, in("ecx") 0xC0000101u32) };
    return d;
}

fn get_per_cpu_data() -> &'static mut PerCpuData {
    let address: usize;
    unsafe {
        asm!("mov {}, gs:0",out(reg) address);
        &mut *(address as *mut PerCpuData)
    }
}
