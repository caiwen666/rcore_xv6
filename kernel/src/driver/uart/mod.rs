mod interface;

use interface::{UartFIFO, UartInterface, UartInterrupt};

pub struct Uart {
    interface: UartInterface,
}

impl Uart {
    /// 创建一个 Uart 实例，会自动初始化
    ///
    /// # Parameters
    ///
    /// - `base_addr`: Uart 的基地址
    ///
    /// # Returns
    ///
    /// 返回一个 Uart 实例
    pub fn new(base_addr: usize) -> Self {
        let uart = Self {
            interface: UartInterface::new(base_addr),
        };
        uart.init();
        uart
    }

    fn init(&self) {
        // 关闭所有中断
        self.interface.set_interrupt(UartInterrupt::new());
        // 设置波特率为 38.4k，分频系数需要设置为 3
        self.interface.set_baud_rate(3);
        // 开启 FIFO 并清空 FIFO 缓冲区
        self.interface
            .set_fifo(UartFIFO::new().set_enable().set_clear());
        // 开启中断
        self.interface
            .set_interrupt(UartInterrupt::new().set_tx_enable().set_rx_enable());
    }

    // TODO 增加缓冲区
    pub fn put(&self, ch: u8) {
        self.interface.put(ch);
    }
}
