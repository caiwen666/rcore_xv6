pub struct RingBuffer<T, const SIZE: usize> {
    data: [T; SIZE],
    head: usize,
    tail: usize,
}

impl<T: Copy + Default, const SIZE: usize> RingBuffer<T, SIZE> {
    /// # Panics
    ///
    /// 由于 RingBuffer 的实现，SIZE 必须至少为 2，否则会 panic
    pub fn new() -> Self {
        assert!(SIZE >= 2, "RingBuffer: SIZE must be at least 2!");
        Self {
            data: [T::default(); SIZE],
            head: 0,
            tail: 0,
        }
    }

    /// 添加元素
    ///
    /// # Panics
    ///
    /// 如果缓冲区已满，则 panic
    pub fn push(&mut self, item: T) {
        if self.is_full() {
            panic!("RingBuffer is full!");
        }
        self.data[self.head] = item;
        self.head = (self.head + 1) % SIZE;
    }

    /// 弹出元素
    ///
    /// # Returns
    ///
    /// 如果缓冲区为空，则返回 None
    pub fn pop(&mut self) -> Option<T> {
        if self.head == self.tail {
            None
        } else {
            let item = self.data[self.tail];
            self.tail = (self.tail + 1) % SIZE;
            Some(item)
        }
    }

    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    pub fn is_full(&self) -> bool {
        (self.head + 1) % SIZE == self.tail
    }
}
