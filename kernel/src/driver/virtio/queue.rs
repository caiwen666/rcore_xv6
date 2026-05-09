use crate::{
    arch::MMArch,
    driver::virtio::transport::Transport,
    mm::{
        MemoryManagementArch,
        address::PhysAddr,
        allocator::{PageFrame, alloc_frame},
    },
};
use bitflags::bitflags;
use core::{
    ptr::NonNull,
    sync::atomic::{AtomicU16, Ordering},
};

#[repr(C)]
pub struct Descriptor {
    addr: u64,
    len: u32,
    flags: DescFlags,
    next: u16,
}

impl Descriptor {
    pub fn new(addr: u64, len: u32, flags: DescFlags, next: u16) -> Self {
        Self {
            addr,
            len,
            flags,
            next,
        }
    }
}

bitflags! {
    pub struct DescFlags: u16 {
        const NEXT = 1 << 0;
        const WRITE = 1 << 1;
    }
}

#[repr(C)]
pub struct AvailRing<const SIZE: usize> {
    /// 总是为 0
    flags: AtomicU16,
    /// 由驱动递增，表示已经提交的描述符的数量
    idx: AtomicU16,
    ring: [u16; SIZE],
    _unused: AtomicU16,
}

#[repr(C)]
pub struct UsedRing<const SIZE: usize> {
    /// 总是为 0
    flags: AtomicU16,
    /// 由设备递增，表示已经处理完毕的描述符的数量
    idx: AtomicU16,
    ring: [UsedElem; SIZE],
    _unused: AtomicU16,
}

#[repr(C)]
struct UsedElem {
    id: u32,
    len: u32,
}

/// 根据队列长度，求出其描述符表，avail ring 和 used ring 的大小
fn queue_part_sizes(queue_size: u16) -> (usize, usize, usize) {
    let queue_size = queue_size as usize;
    let desc = size_of::<Descriptor>() * queue_size;
    let avail = size_of::<u16>() * (queue_size + 3);
    let used = size_of::<u16>() * 3 + size_of::<UsedElem>() * queue_size;
    (desc, avail, used)
}

struct VirtQueueLayout {
    dma: PageFrame,
    avail_offset: usize,
    used_offset: usize,
}

impl VirtQueueLayout {
    fn new(queue_size: u16) -> Self {
        let (desc, avail, used) = queue_part_sizes(queue_size);
        let part1_size = (desc + avail).div_ceil(MMArch::PAGE_SIZE) * MMArch::PAGE_SIZE;
        let part2_size = used.div_ceil(MMArch::PAGE_SIZE) * MMArch::PAGE_SIZE;
        let mut dma = alloc_frame((part1_size + part2_size) / MMArch::PAGE_SIZE)
            .expect("Failed to allocate DMA page frame");
        dma.clear();
        Self {
            dma,
            avail_offset: desc,
            used_offset: part1_size,
        }
    }

    fn descriptors_paddr(&self) -> PhysAddr {
        self.dma.addr()
    }

    fn avail_paddr(&self) -> PhysAddr {
        self.dma.addr() + self.avail_offset
    }

    fn used_paddr(&self) -> PhysAddr {
        self.dma.addr() + self.used_offset
    }
}

/// VirtIO 队列
///
/// SIZE 必须是 2 的幂次，且不能大于 u16::MAX
pub struct VirtQueue<const SIZE: usize> {
    #[expect(unused)]
    layout: VirtQueueLayout,
    desc: NonNull<[Descriptor]>,
    avail: NonNull<AvailRing<SIZE>>,
    used: NonNull<UsedRing<SIZE>>,
    /// 空闲的描述符的数量
    free_num: u16,
    /// 空闲的描述符链表的头结点
    free_head: u16,
    /// 队列的索引
    queue_idx: u16,
    /// 当前已经处理了几个 used ring 中的描述符
    last_used_idx: u16,
}

impl<const SIZE: usize> VirtQueue<SIZE> {
    #[expect(unused)]
    const SIZE_OK: () = assert!(SIZE.is_power_of_two() && SIZE <= u16::MAX as usize);

    /// - `idx`：队列的索引
    pub fn new<T: Transport>(transport: &mut T, idx: u16) -> Self {
        if core::hint::unlikely(transport.max_queue_size(idx) < SIZE as u32) {
            panic!(
                "Queue size is too large: {} < {}",
                transport.max_queue_size(idx),
                SIZE
            );
        }
        let layout = VirtQueueLayout::new(SIZE as u16);
        transport.queue_set(idx, SIZE as u32, layout.descriptors_paddr());
        let desc = NonNull::slice_from_raw_parts(
            NonNull::from_mut(layout.descriptors_paddr().get_mut::<Descriptor>()),
            SIZE,
        );
        let avail = NonNull::from_mut(layout.avail_paddr().get_mut::<AvailRing<SIZE>>());
        let used = NonNull::from_mut(layout.used_paddr().get_mut::<UsedRing<SIZE>>());
        for i in 0..(SIZE - 1) {
            unsafe {
                (*desc.as_ptr())[i].next = (i + 1) as u16;
            }
        }
        Self {
            layout,
            desc,
            avail,
            used,
            queue_idx: idx,
            last_used_idx: 0,
            free_num: SIZE as u16,
            free_head: 0,
        }
    }

    /// 如果没有空闲的描述符了，就返回 None
    pub fn alloc_descriptor(&mut self) -> Option<u16> {
        if self.free_num == 0 {
            None
        } else {
            let head = self.free_head;
            self.free_head = unsafe { (*self.desc.as_ptr())[head as usize].next };
            self.free_num -= 1;
            Some(head)
        }
    }

    /// 从 used ring 中弹出一个描述符链。
    /// 如果 used 为空，则返回 None，
    /// 否则，则返回链条的头结点
    pub fn recycle_descriptor(&mut self) -> Option<u16> {
        if self.last_used_idx == unsafe { (*self.used.as_ptr()).idx.load(Ordering::Acquire) } {
            return None;
        }
        let used_slot = self.last_used_idx & (SIZE as u16 - 1);
        let head = unsafe { (*self.used.as_ptr()).ring[used_slot as usize].id as u16 };
        self.last_used_idx = self.last_used_idx.wrapping_add(1);

        let mut tail = head;
        let mut chain_len: u16 = 1;
        while unsafe {
            (*self.desc.as_ptr())[tail as usize]
                .flags
                .contains(DescFlags::NEXT)
        } {
            tail = unsafe { (*self.desc.as_ptr())[tail as usize].next };
            chain_len += 1;
        }

        unsafe {
            (*self.desc.as_ptr())[tail as usize].next = self.free_head;
        }
        self.free_head = head;
        self.free_num += chain_len;

        Some(head)
    }

    pub fn write_descriptor(&mut self, idx: u16, desc: Descriptor) {
        unsafe {
            (*self.desc.as_ptr())[idx as usize] = desc;
        }
    }

    pub fn request(&mut self, desc_idx: u16, transport: &mut impl Transport) {
        unsafe {
            (*self.avail.as_ptr()).ring[((*self.avail.as_ptr()).idx.load(Ordering::Acquire)
                & (SIZE as u16 - 1)) as usize] = desc_idx;
            (*self.avail.as_ptr()).idx.fetch_add(1, Ordering::Release);
        }
        transport.notify(self.queue_idx);
    }
}

// SAFETY: None of the virt queue resources are tied to a particular thread.
unsafe impl<const SIZE: usize> Send for VirtQueue<SIZE> {}

// SAFETY: A `&VirtQueue` only allows reading from the various pointers it contains, so there is no
// data race.
unsafe impl<const SIZE: usize> Sync for VirtQueue<SIZE> {}
