//! ACPIテーブル解析用コード
//!
//! ここではLocal APIC idのリスト取得に必要なMADTと
//! ACPI PM Timerの取得に必要なFACPテーブルの解析のみを行っています。
//! なお、チャックサムの確認は省略しています。

use super::acpi_pm_timer::AcpiPmTimer;

#[repr(C, packed)]
struct RSDP {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
    length: u32,
    xsdt_address: u64,
    ex_checksum: u32,
    reserved: [u8; 3],
}

#[repr(C, packed)]
struct MADT {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: [u8; 4],
    creator_revision: [u8; 4],
    flags: u32,
    local_interrupt_controller_address: u32,
    /* interrupt_controller_structure: [struct; n] */
}

#[repr(C, packed)]
struct FADT {
    signature: [u8; 4],
    length: u32,
    major_version: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: [u8; 4],
    creator_revision: [u8; 4],
    ignore: [u8; 76 - 36],
    pm_tmr_block: u32,
    ignore2: [u8; 112 - 80],
    flags: u32,
    ignore3: [u8; 276 - 116],
}

#[derive(Clone)]
pub struct ApicIdList {
    base_address: usize,
    length: usize,
    pointer: usize,
}

fn get_xsdt(rsdp_address: usize) -> Option<usize> {
    let rsdp = unsafe { &*(rsdp_address as *const RSDP) };
    if rsdp.revision < 2 {
        None
    } else {
        Some(rsdp.xsdt_address as usize)
    }
}

fn get_rsdt(rsdp_address: usize) -> usize {
    unsafe { (&*(rsdp_address as *const RSDP)).rsdt_address as usize }
}

fn get_entry(address: usize, index: usize, is_xsdt: bool) -> Option<usize> {
    let pointer_size = if is_xsdt { 8 } else { 4 };
    let length = { unsafe { *((address + 4) as *const u32) } } as usize;
    if (length - 0x24) > index * pointer_size {
        Some(unsafe { *((address + 0x24 + index * pointer_size) as *const u32) } as usize)
    } else {
        None
    }
}

fn get_madt(rsdp_address: usize) -> Option<usize> {
    let (address, is_xsdt) = if let Some(xsdt_address) = get_xsdt(rsdp_address) {
        (xsdt_address, true)
    } else {
        (get_rsdt(rsdp_address), false)
    };
    let mut index = 0;
    while let Some(entry_address) = get_entry(address, index, is_xsdt) {
        if unsafe { *(entry_address as *const [u8; 4]) }
            == ['A' as u8, 'P' as u8, 'I' as u8, 'C' as u8]
        {
            return Some(entry_address);
        }
        index += 1;
    }
    None
}

pub fn get_apic_id_list(rsdp_address: usize) -> Option<ApicIdList> {
    let madt_address = if let Some(madt) = get_madt(rsdp_address) {
        madt
    } else {
        return None;
    };
    let madt = unsafe { &*(madt_address as *const MADT) };
    let length = madt.length as usize - core::mem::size_of::<MADT>();
    let base_address = madt_address + core::mem::size_of::<MADT>();
    Some(ApicIdList {
        base_address,
        length,
        pointer: 0,
    })
}

pub fn get_acpi_pm_timer(rsdp_address: usize) -> Option<AcpiPmTimer> {
    let (address, is_xsdt) = if let Some(xsdt_address) = get_xsdt(rsdp_address) {
        (xsdt_address, true)
    } else {
        (get_rsdt(rsdp_address), false)
    };

    let mut index = 0;
    while let Some(entry_address) = get_entry(address, index, is_xsdt) {
        if unsafe { *(entry_address as *const [u8; 4]) }
            == ['F' as u8, 'A' as u8, 'C' as u8, 'P' as u8]
        {
            let fadt = unsafe { &*(entry_address as *const FADT) };
            return Some(AcpiPmTimer::new(
                fadt.pm_tmr_block as usize,
                ((fadt.flags >> 8) & 1) != 0,
            ));
        }
        index += 1;
    }
    None
}

impl ApicIdList {
    pub const fn const_new() -> Self {
        Self {
            base_address: 0,
            length: 0,
            pointer: 0,
        }
    }
}

impl Iterator for ApicIdList {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item> {
        if self.pointer >= self.length {
            return None;
        }
        let entry_address = self.base_address + self.pointer;
        let record_type = unsafe { *(entry_address as *const u8) };
        let record_length = unsafe { *((entry_address + 1) as *const u8) };
        self.pointer += record_length as usize;
        match record_type {
            0 => {
                if unsafe { *((entry_address + 4) as *const u32) & 1 } == 1 {
                    /* Enabled */
                    Some(unsafe { *((entry_address + 3) as *const u8) } as u32)
                } else {
                    self.next()
                }
            }
            9 => {
                if unsafe { *((entry_address + 8) as *const u32) & 1 } == 1 {
                    /* Enabled */
                    Some(unsafe { *((entry_address + 4) as *const u32) })
                } else {
                    self.next()
                }
            }
            _ => self.next(),
        }
    }
}
