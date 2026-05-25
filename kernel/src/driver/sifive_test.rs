pub struct SiFiveTest {
    base_addr: usize,
}

impl SiFiveTest {
    pub(super) const fn new(base_addr: usize) -> Self {
        Self { base_addr }
    }
}

/// 关机原因
pub enum ShutdownReason {
    /// 异常关机
    Failure,
    /// 正常关机
    #[expect(unused)]
    Normal,
}

impl SiFiveTest {
    /// 关机
    ///
    /// # Parameters
    ///
    /// - `reason`: 关机原因
    /// - `code`: 关机代码
    pub fn shutdown(&self, reason: ShutdownReason, code: u16) -> ! {
        let status: u16 = match reason {
            ShutdownReason::Failure => 0x3333,
            ShutdownReason::Normal => 0x5555,
        };
        let value = ((code as u32) << 16) | status as u32;
        unsafe {
            core::ptr::write_volatile(self.base_addr as *mut u32, value);
        }
        loop {
            core::hint::spin_loop();
        }
    }
}
