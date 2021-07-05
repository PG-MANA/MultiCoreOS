pub fn get_apic_id() -> u8 {
    (unsafe { (core::ptr::read_volatile((0xfee00020usize) as *const u32) >> 24) & 0xff }) as u8
}

pub fn send_interrupt_command(
    destination: u32,
    delivery_mode: u8,
    trigger_mode: u8,
    level: u8,
    vector: u8,
) {
    assert!(delivery_mode < 8);
    let mut data: u64 = ((trigger_mode as u64) << 15)
        | ((level as u64) << 14)
        | ((delivery_mode as u64) << 8)
        | (vector as u64);
    assert!(destination <= 0xff);
    data |= (destination as u64) << 56;
    let high = (data >> 32) as u32;
    let low = data as u32;
    unsafe {
        core::ptr::write_volatile((0xfee00000usize + (0x30 + 1) * 0x10) as *mut u32, high);
        core::ptr::write_volatile((0xfee00000usize + (0x30) * 0x10) as *mut u32, low);
    }
}
