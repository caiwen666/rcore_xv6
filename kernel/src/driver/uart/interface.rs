// 控制寄存器
const LCR: usize = 3;
const LCR_BAUD_LATCH: u8 = 1 << 7;
const LCR_EIGHT_BITS: u8 = 3;
// 中断寄存器
const IER: usize = 1;
const IER_RX_ENABLE: u8 = 1 << 0;
const IER_TX_ENABLE: u8 = 1 << 1;
// FIFO 控制寄存器
const FCR: usize = 2;
const FCR_FIFO_CLEAR: u8 = 3 << 1;
const FCR_FIFO_ENABLE: u8 = 1 << 0;
// 状态寄存器
const LSR: usize = 5;
const LSR_TX_IDLE: u8 = 1 << 5;
const LSR_RX_IDLE: u8 = 1 << 0;

pub struct UartInterface {
    base_addr: usize,
}

impl UartInterface {
    pub fn new(base_addr: usize) -> Self {
        Self { base_addr }
    }
}

impl UartInterface {
    fn write_reg(&self, offset: usize, value: u8) {
        unsafe {
            core::ptr::write_volatile((self.base_addr + offset) as *mut u8, value);
        }
    }

    fn read_reg(&self, offset: usize) -> u8 {
        unsafe { core::ptr::read_volatile((self.base_addr + offset) as *const u8) }
    }
}

pub struct UartInterrupt {
    bits: u8,
}

impl UartInterrupt {
    pub fn new() -> Self {
        Self::from_bits(0)
    }

    pub fn from_bits(bits: u8) -> Self {
        Self { bits }
    }

    pub fn bits(&self) -> u8 {
        self.bits
    }
}

impl UartInterrupt {
    /// 开启发送中断
    pub fn set_tx_enable(mut self) -> Self {
        self.bits |= IER_TX_ENABLE;
        self
    }

    /// 开启接收中断
    pub fn set_rx_enable(mut self) -> Self {
        self.bits |= IER_RX_ENABLE;
        self
    }
}

pub struct UartFIFO {
    bits: u8,
}

impl UartFIFO {
    pub fn new() -> Self {
        Self::from_bits(0)
    }

    pub fn from_bits(bits: u8) -> Self {
        Self { bits }
    }

    pub fn bits(&self) -> u8 {
        self.bits
    }
}

impl UartFIFO {
    /// 是否开启 FIFO
    pub fn set_enable(mut self) -> Self {
        self.bits |= FCR_FIFO_ENABLE;
        self
    }

    /// 清空 FIFO 缓冲区，包括接收缓冲区和发送缓冲区
    pub fn set_clear(mut self) -> Self {
        self.bits |= FCR_FIFO_CLEAR;
        self
    }
}

impl UartInterface {
    /// 开启 Uart 的哪些中断
    pub fn set_interrupt(&self, interrupt: UartInterrupt) {
        self.write_reg(IER, interrupt.bits());
    }

    /// 设置波特率
    ///
    /// 波特率 = 系统时钟频率 / (16 * 分频系数)
    ///
    /// # Parameters
    ///
    /// - `divisor`: 分频系数
    ///
    /// # Notes
    ///
    /// 设置为波特率后会自动退出设置波特率的模式，并设置数据位为8位
    pub fn set_baud_rate(&self, divisor: u16) {
        // 进入设置波特率的模式
        self.write_reg(LCR, LCR_BAUD_LATCH);
        // 分别设置分频系数的低8位和高8位
        let low = (divisor & 0xFF) as u8;
        let high = ((divisor >> 8) & 0xFF) as u8;
        self.write_reg(0, low);
        self.write_reg(1, high);
        // 退出设置波特率的模式，并设置数据位为8位
        self.write_reg(LCR, LCR_EIGHT_BITS);
    }

    /// 设置 FIFO
    pub fn set_fifo(&self, fifo: UartFIFO) {
        self.write_reg(FCR, fifo.bits());
    }

    /// 是否可以发送数据
    #[inline]
    pub fn tx_idle(&self) -> bool {
        self.read_reg(LSR) & LSR_TX_IDLE != 0
    }

    /// 是否可以接收数据
    #[inline]
    pub fn rx_ready(&self) -> bool {
        self.read_reg(LSR) & LSR_RX_IDLE != 0
    }

    /// 发送一个字节
    ///
    /// # Preconditions
    ///
    /// 必须确保 UART 可以发送数据，即 [Self::tx_idle()] 为 true，
    /// 才能调用该函数，否则会出现未定义行为
    pub fn put(&self, ch: u8) {
        unsafe {
            core::ptr::write_volatile(self.base_addr as *mut u8, ch);
        }
    }

    /// 读入一个字节
    ///
    /// # Preconditions
    ///
    /// 必须确保 UART 可以接收数据，即 [Self::rx_ready()] 为 true，
    /// 才能调用该函数，否则会出现未定义行为
    pub fn get(&self) -> u8 {
        unsafe { core::ptr::read_volatile(self.base_addr as *const u8) }
    }
}
