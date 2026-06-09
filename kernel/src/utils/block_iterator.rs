/// 块迭代器
///
/// 用于比较方便地一块一块地去迭代
pub struct BlockIterator {
    block_size: usize,
    offset: usize,
    size: usize,
    /// 当前遍历到的位置
    current_pos: usize,
    block_start: usize,
    block_end: usize,
}

impl BlockIterator {
    /// 创建一个块迭代器
    ///
    /// # Parameters
    ///
    /// - `block_size`: 块大小
    /// - `offset`: 遍历的起始偏移量
    /// - `size`: 遍历的大小
    pub fn new(block_size: usize, offset: usize, size: usize) -> Self {
        Self {
            block_size,
            offset,
            size,
            current_pos: offset,
            block_start: offset / block_size,
            block_end: (offset + size).div_ceil(block_size),
        }
    }
}

pub struct BlockIteratorItem {
    block_id: usize,
    offset: usize,
    size: usize,
}

impl BlockIteratorItem {
    /// 当前遍历到的块编号，从 0 开始
    pub fn block_id(&self) -> usize {
        self.block_id
    }

    /// 当前遍历到的块的块内的偏移量
    ///
    /// 一般为 0，在最开头的散块会非 0
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// 在当前遍历到的块内读取的数据大小
    ///
    /// 一般为块大小，在最开头和最末尾的散块中会小于块大小
    pub fn size(&self) -> usize {
        self.size
    }
}

impl Iterator for BlockIterator {
    type Item = BlockIteratorItem;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_pos >= self.offset + self.size {
            None
        } else {
            let block_id = self.current_pos / self.block_size;
            let offset = if block_id == self.block_start {
                self.current_pos % self.block_size
            } else {
                0
            };
            let size = if block_id == self.block_start {
                (self.block_size - offset).min(self.size)
            } else if block_id == self.block_end {
                self.offset + self.size - self.current_pos
            } else {
                self.block_size - offset
            };
            self.current_pos += size;
            Some(BlockIteratorItem {
                block_id,
                offset,
                size,
            })
        }
    }
}
