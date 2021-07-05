//! メモリ管理用モジュール
//!
//! Multiboot Informationのメモリ関係の情報をもとに空きメモリを管理し
//! 貸し出してます。簡略化のためfreeの実装はしてません。

use core::mem;

#[derive(Clone)]
#[repr(C)]
struct MemoryMapEntry {
    pub addr: u64,
    pub length: u64,
    pub m_type: u32,
    pub reserved: u32,
}

#[repr(C)]
#[allow(dead_code)]
pub struct MultibootTagMemoryMap {
    s_type: u32,
    size: u32,
    entry_size: u32,
    entry_version: u32,
}

#[repr(C)]
pub struct MultibootTagElfSections {
    s_type: u32,
    size: u32,
    num: u32,
    entsize: u32,
    shndx: u32,
}

#[repr(C)]
pub struct ElfSection {
    section_name: u32,
    section_type: u32,
    section_flags: usize,
    section_addr: usize,
    section_offset: usize,
    section_size: usize,
    section_link: u32,
    section_info: u32,
    section_addralign: usize,
    section_entry_size: usize,
}

pub struct MemoryManager {
    address: usize,
    num_of_entries: u32,
}

impl MemoryManager {
    pub const fn const_new() -> Self {
        Self {
            address: 0,
            num_of_entries: 0,
        }
    }

    pub fn new(map: &MultibootTagMemoryMap, elf_info: &MultibootTagElfSections) -> Self {
        let m = Self {
            num_of_entries: ((map.size - mem::size_of::<MultibootTagMemoryMap>() as u32)
                / map.entry_size),
            address: map as *const _ as usize + mem::size_of::<MultibootTagMemoryMap>(),
        };
        for i in 0..(elf_info.num as usize) {
            let entry = unsafe {
                &*((elf_info as *const _ as usize
                    + mem::size_of::<MultibootTagElfSections>()
                    + i * mem::size_of::<ElfSection>()) as *const ElfSection)
            };
            if entry.section_flags & 2 == 0 {
                continue;
            }
            m.reserve(entry.section_addr, entry.section_size);
        }
        m
    }

    fn reserve(&self, address: usize, size: usize) {
        for i in 0..(self.num_of_entries as usize) {
            let entry = unsafe {
                &mut *((self.address + i * mem::size_of::<MemoryMapEntry>()) as *mut MemoryMapEntry)
            };
            if entry.m_type != 1 {
                continue;
            }
            if (entry.addr as usize) < address
                && ((entry.addr + entry.length) as usize) > address + size
            {
                entry.length -= (size as u64) + ((address as u64) - entry.addr);
                entry.addr = (address + size) as u64;
                return;
            }
        }
    }

    pub fn alloc(&self, size: usize) -> Option<usize> {
        for i in 0..(self.num_of_entries as usize) {
            let entry = unsafe {
                &mut *((self.address + i * mem::size_of::<MemoryMapEntry>()) as *mut MemoryMapEntry)
            };
            if entry.m_type != 1 {
                continue;
            }
            if entry.length as usize >= size {
                if ((entry.addr as usize)..((entry.addr as usize) + size)).contains(&self.address) {
                    if (entry.addr + entry.length) as usize
                        - (self.address
                            + (self.num_of_entries as usize) * mem::size_of::<MemoryMapEntry>())
                        < size
                    {
                        continue;
                    }
                    let allocated_address = self.address
                        + (self.num_of_entries as usize) * mem::size_of::<MemoryMapEntry>();
                    entry.addr = (allocated_address + size) as u64;
                    entry.length = ((entry.addr + entry.length) as usize
                        - (self.address
                            + (self.num_of_entries as usize) * mem::size_of::<MemoryMapEntry>())
                        - size) as u64;
                    return Some(allocated_address);
                }
                let allocated_address = entry.addr as usize;
                entry.addr += size as u64;
                entry.length -= size as u64;
                return Some(allocated_address);
            }
        }
        None
    }

    pub fn alloc_with_align(&self, size: usize, align: usize) -> Option<usize> {
        let mut address = if size < align {
            self.alloc(align)?
        } else {
            self.alloc(size + align)?
        };
        if address != 0 {
            address += align - (address & (align - 1));
        }
        Some(address)
    }
}
