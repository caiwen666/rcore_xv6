pub struct PLIC {
    base_addr: usize,
}

impl PLIC {
    pub const fn new(base_addr: usize) -> Self {
        Self { base_addr }
    }

    /// 设置某个设备的优先级
    ///
    /// 优先级为 0 时表示不接收这个设备的中断，不为 0 表示接收
    pub fn set_priority(&self, irq: u32, priority: u32) {
        let priority_addr = self.base_addr + irq as usize * 4;
        unsafe {
            core::ptr::write_volatile(priority_addr as *mut u32, priority);
        }
    }

    /// 设置 CPU 在 supervisor 模式下能接收到哪些中断
    pub fn set_supervisor_enable(&self, cpu_id: usize, bits: u32) {
        let addr = self.base_addr + 0x2080 + cpu_id * 0x100;
        unsafe {
            core::ptr::write_volatile(addr as *mut u32, bits);
        }
    }

    /// 设置 CPU 在 supervisor 模式下接收中断的优先级阈值
    pub fn set_supervisor_threshold(&self, cpu_id: usize, threshold: u32) {
        let addr = self.base_addr + 0x201000 + cpu_id * 0x2000;
        unsafe {
            core::ptr::write_volatile(addr as *mut u32, threshold);
        }
    }

    /// 获取指定 CPU 当前接收到的中断
    pub fn get_current_interrupt(&self, cpu_id: usize) -> u32 {
        let addr = self.base_addr + 0x201004 + cpu_id * 0x2000;
        unsafe { core::ptr::read_volatile(addr as *const u32) }
    }

    /// 告诉 PLIC 某个中断已经处理完毕
    pub fn complete_interrupt(&self, cpu_id: usize, irq: u32) {
        let addr = self.base_addr + 0x201004 + cpu_id * 0x2000;
        unsafe { core::ptr::write_volatile(addr as *mut u32, irq); }
    }
}
