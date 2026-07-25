use crate::driver::cpu::set_online_cpu_mask;

/// 解析设备树并进行设备的初始化
pub fn init_from_dtb(dtb_pa: usize) {
    let fdt = unsafe { fdt::Fdt::from_ptr(dtb_pa as *const u8).expect("invalid DTB") };

    // 解析 CPU
    let mut cpu_mask = 0usize;
    for cpu in fdt.cpus() {
        if let Some(status) = cpu.property("status").and_then(|p| p.as_str())
            && status != "okay"
            && status != "ok"
        {
            continue;
        }
        // 不考虑超线程，所以只取第一个就够
        let hart_id = cpu.ids().first();
        let bit = 1usize << hart_id;
        if cpu_mask & bit == 0 {
            cpu_mask |= bit;
        }
    }
    unsafe {
        set_online_cpu_mask(cpu_mask);
    }
}
