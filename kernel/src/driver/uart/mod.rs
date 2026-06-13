mod interface;

use crate::{
    sync::{
        condvar::Condvar,
        spin::{SpinMutex, SpinMutexGuard},
    },
    utils::RingBuffer,
};
use interface::{UartFIFO, UartInterface, UartInterrupt};

const BUF_SIZE: usize = 512;

pub struct Uart {
    interface: UartInterface,
    put_buf: SpinMutex<RingBuffer<u8, BUF_SIZE>>,
    put_condvar: Condvar,
    get_buf: SpinMutex<RingBuffer<u8, BUF_SIZE>>,
    get_condvar: Condvar,
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
            put_buf: SpinMutex::new(RingBuffer::new(), "uart_put"),
            put_condvar: Condvar::new(),
            get_buf: SpinMutex::new(RingBuffer::new(), "uart_get"),
            get_condvar: Condvar::new(),
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

    /// 输出一个字节
    ///
    /// 该函数会一直阻塞等待，直到数据被成功输出为止。
    pub fn put_sync(&self, ch: u8) {
        let _guard = self.put_buf.lock();
        loop {
            if self.interface.tx_idle() {
                break;
            }
        }
        self.interface.put(ch);
    }

    /// 输出一个字节
    ///
    /// 字节不会立刻输出，而是会先放入内核的缓冲区中。如果缓冲区满，则当前线程会被挂起。
    ///
    /// # Panic
    ///
    /// 必须在线程中调用该函数，否则会 panic
    pub fn put(&self, ch: u8) {
        let mut buf = self.put_buf.lock();
        loop {
            if buf.is_full() {
                buf = self.put_condvar.wait(buf);
            } else {
                buf.push(ch);
                self.start_put(buf);
                break;
            }
        }
    }

    /// 开始发送数据，直到缓冲区为空或是 UART 繁忙为止
    fn start_put(&self, mut buf: SpinMutexGuard<'_, RingBuffer<u8, BUF_SIZE>>) {
        while !buf.is_empty() {
            // 如果当前 UART 繁忙，则直接返回
            if !self.interface.tx_idle() {
                break;
            }
            let ch = buf.pop().unwrap();
            self.interface.put(ch);
        }
        self.put_condvar.notify_all();
    }

    /// 读入一个字节
    ///
    /// **会堵塞**
    pub fn get(&self) -> u8 {
        let mut buf = self.get_buf.lock();
        loop {
            if buf.is_empty() {
                buf = self.get_condvar.wait(buf);
            } else {
                return buf.pop().unwrap();
            }
        }
    }

    /// 开始接收数据，直到缓冲区为满或是没有可读入的数据为止
    fn start_get(&self, mut buf: SpinMutexGuard<'_, RingBuffer<u8, BUF_SIZE>>) {
        while !buf.is_full() {
            if !self.interface.rx_ready() {
                break;
            }
            let ch = self.interface.get();
            buf.push(ch);
        }
        self.get_condvar.notify_all();
    }

    /// 处理中断
    ///
    /// 当 UART 可以发送数据，或是接收到了数据时，会调用触发中断
    pub fn handle_interrupt(&self) {
        let get_buf = self.get_buf.lock();
        self.start_get(get_buf);
        let put_buf = self.put_buf.lock();
        self.start_put(put_buf);
    }
}
