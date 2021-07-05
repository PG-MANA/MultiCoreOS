#![no_std]
#![feature(const_fn_fn_ptr_basics)]
#![feature(global_asm)]
#![feature(lang_items)]
#![feature(asm)]
#![feature(panic_info_message)]

#[macro_use]
mod print;
mod acpi;
mod acpi_pm_timer;
mod ap;
mod asm;
mod local_apic;
mod memory;

use acpi::{get_acpi_pm_timer, get_apic_id_list, ApicIdList};
use acpi_pm_timer::AcpiPmTimer;
use ap::init_ap;
use memory::{MemoryManager, MultibootTagElfSections, MultibootTagMemoryMap};
use print::PRINT_MANAGER;

use core::panic;

#[repr(C)]
struct MultibootTag {
    s_type: u32,
    size: u32,
}

#[repr(C)]
#[allow(dead_code)]
pub struct MultibootTagFrameBuffer {
    s_type: u32,
    size: u32,
    frame_buffer_addr: u64,
    frame_buffer_pitch: u32,
    frame_buffer_width: u32,
    frame_buffer_height: u32,
    frame_buffer_bpp: u8,
    frame_buffer_type: u8,
    /* https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html 3.6.12 Framebuffer info */
    reserved: u8,
    /* color_info is ignored */
}

#[repr(C)]
struct MultibootTagModule {
    s_type: u32,
    size: u32,
    mod_start: u32,
    mod_end: u32,
    string: u8,
}

static mut MEMORY_MANAGER: MemoryManager = MemoryManager::const_new();
static mut APIC_ID_LIST: ApicIdList = ApicIdList::const_new();
static mut ACPI_PM_TIMER: AcpiPmTimer = AcpiPmTimer::const_new();

#[no_mangle]
extern "C" fn boot_main(multiboot_info_address: usize) -> ! {
    init(multiboot_info_address);
    println!("Setup application processors!!");
    unsafe { init_ap(APIC_ID_LIST.clone(), &ACPI_PM_TIMER) };
    println!("Setup succeeded!!");
    loop {
        unsafe { asm!("hlt") };
    }
}

fn init(multiboot_info_address: usize) {
    if multiboot_info_address & 7 != 0 {
        panic!("Invalid Multiboot information address");
    }
    let mut tag = multiboot_info_address + 8;

    let mut elf_info_address = 0usize;
    let mut memory_map_info_address = 0usize;
    let mut frame_buffer_info_address = 0usize;
    let mut font_data_address = 0usize;
    let mut font_data_size = 0usize;
    let mut new_rsdp_address = 0usize;
    let mut old_rsdp_address = 0usize;

    const TAG_TYPE_END: u32 = 0;
    const TAG_TYPE_MODULE: u32 = 3;
    const TAG_TYPE_MMAP: u32 = 6;
    const TAG_TYPE_FRAMEBUFFER: u32 = 8;
    const TAG_TYPE_ELF_SECTIONS: u32 = 9;
    const TAG_TYPE_ACPI_OLD: u32 = 14;
    const TAG_TYPE_ACPI_NEW: u32 = 15;

    loop {
        match unsafe { (*(tag as *const MultibootTag)).s_type } {
            TAG_TYPE_END => {
                break;
            }
            TAG_TYPE_MMAP => {
                memory_map_info_address = tag;
            }
            TAG_TYPE_MODULE => {
                let module_info = unsafe { &*(tag as *const MultibootTagModule) };
                if core::str::from_utf8(unsafe {
                    core::slice::from_raw_parts(
                        &module_info.string,
                        module_info.size as usize - 16 - 1, /*\0*/
                    )
                })
                .unwrap_or("")
                    == "font.pf2"
                {
                    font_data_address = module_info.mod_start as usize;
                    font_data_size = module_info.mod_end as usize - font_data_address;
                }
            }
            TAG_TYPE_FRAMEBUFFER => {
                frame_buffer_info_address = tag;
            }
            TAG_TYPE_ELF_SECTIONS => {
                elf_info_address = tag;
            }
            TAG_TYPE_ACPI_NEW => {
                new_rsdp_address = tag + 8;
            }
            TAG_TYPE_ACPI_OLD => {
                old_rsdp_address = tag + 8;
            }
            _ => {}
        }
        tag += (unsafe { (*(tag as *const MultibootTag)).size } as usize + 7) & !7;
    }

    if frame_buffer_info_address != 0 {
        let frame_buffer_info =
            unsafe { &*(frame_buffer_info_address as *const MultibootTagFrameBuffer) };
        unsafe {
            PRINT_MANAGER.init(
                frame_buffer_info.frame_buffer_addr as usize,
                frame_buffer_info.frame_buffer_width as usize,
                frame_buffer_info.frame_buffer_height as usize,
                frame_buffer_info.frame_buffer_bpp as u8,
                font_data_address,
                font_data_size,
            )
        };
    }

    assert_ne!(elf_info_address, 0);
    assert_ne!(memory_map_info_address, 0);

    unsafe {
        MEMORY_MANAGER = MemoryManager::new(
            &*(memory_map_info_address as *const MultibootTagMemoryMap),
            &*(elf_info_address as *const MultibootTagElfSections),
        );
    }

    if new_rsdp_address == 0 && old_rsdp_address == 0 {
        panic!("ACPI is not supported!");
    }

    let rsdp_address = if new_rsdp_address != 0 {
        println!("ACPI 2.0 or later");
        new_rsdp_address
    } else {
        println!("ACPI 1.0");
        old_rsdp_address
    };

    unsafe {
        APIC_ID_LIST = get_apic_id_list(rsdp_address).expect("Cannot get Local APIC ID List!");
        ACPI_PM_TIMER = get_acpi_pm_timer(rsdp_address).expect("Cannot get ACPI PM Timer!");
    }
}

#[panic_handler]
#[no_mangle]
pub fn panic(info: &panic::PanicInfo) -> ! {
    let location = info.location();
    let message = info.message();

    println!("\n!!!! Kernel Panic !!!!");
    if location.is_some() && message.is_some() {
        println!(
            "Line {} in {}\nMessage: {}",
            location.unwrap().line(),
            location.unwrap().file(),
            message.unwrap()
        );
    }
    loop {
        unsafe { asm!("hlt") };
    }
}
