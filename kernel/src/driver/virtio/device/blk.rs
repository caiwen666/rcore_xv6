use crate::{
    driver::virtio::{
        queue::{DescFlags, Descriptor, VirtQueue},
        transport::Transport,
    },
    mm::{KERNEL_SPACE, address::VirtAddr},
    sync::{condvar::Condvar, spin::SpinMutex},
};
use alloc::{boxed::Box, collections::btree_map::BTreeMap};
use bitflags::bitflags;
use core::mem::offset_of;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

const QUEUE_SIZE: u16 = 8;
const SECTOR_SIZE: usize = 512;

bitflags! {
    #[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
    pub struct BlkFeature: u64 {
        /// Device supports request barriers. (legacy)
        const BARRIER       = 1 << 0;
        /// Maximum size of any single segment is in `size_max`.
        const SIZE_MAX      = 1 << 1;
        /// Maximum number of segments in a request is in `seg_max`.
        const SEG_MAX       = 1 << 2;
        /// Disk-style geometry specified in geometry.
        const GEOMETRY      = 1 << 4;
        /// Device is read-only.
        const RO            = 1 << 5;
        /// Block size of disk is in `blk_size`.
        const BLK_SIZE      = 1 << 6;
        /// Device supports scsi packet commands. (legacy)
        const SCSI          = 1 << 7;
        /// Cache flush command support.
        const FLUSH         = 1 << 9;
        /// Device exports information on optimal I/O alignment.
        const TOPOLOGY      = 1 << 10;
        /// Device can toggle its cache between writeback and writethrough modes.
        const CONFIG_WCE    = 1 << 11;
        /// Device supports multiqueue.
        const MQ            = 1 << 12;
        /// Device can support discard command, maximum discard sectors size in
        /// `max_discard_sectors` and maximum discard segment number in
        /// `max_discard_seg`.
        const DISCARD       = 1 << 13;
        /// Device can support write zeroes command, maximum write zeroes sectors
        /// size in `max_write_zeroes_sectors` and maximum write zeroes segment
        /// number in `max_write_zeroes_seg`.
        const WRITE_ZEROES  = 1 << 14;
        /// Device supports providing storage lifetime information.
        const LIFETIME      = 1 << 15;
        /// Device can support the secure erase command.
        const SECURE_ERASE  = 1 << 16;

        // device independent
        const NOTIFY_ON_EMPTY       = 1 << 24; // legacy
        const ANY_LAYOUT            = 1 << 27; // legacy
        const RING_INDIRECT_DESC    = 1 << 28;
        const RING_EVENT_IDX        = 1 << 29;
        const UNUSED                = 1 << 30; // legacy
        const VERSION_1             = 1 << 32; // detect legacy

        // the following since virtio v1.1
        const ACCESS_PLATFORM       = 1 << 33;
        const RING_PACKED           = 1 << 34;
        const IN_ORDER              = 1 << 35;
        const ORDER_PLATFORM        = 1 << 36;
        const SR_IOV                = 1 << 37;
        const NOTIFICATION_DATA     = 1 << 38;
    }
}

#[derive(FromBytes, Immutable, IntoBytes)]
#[repr(C)]
struct BlkConfig {
    /// Number of 512 Bytes sectors
    capacity_low: u32,
    capacity_high: u32,
    // ... ignored
}

/// A VirtIO block device request.
#[repr(C)]
#[derive(Debug, Immutable, IntoBytes, KnownLayout)]
pub struct BlkReq {
    type_: ReqType,
    reserved: u32,
    sector: u64,
}

#[repr(u32)]
#[derive(Debug, Immutable, IntoBytes, KnownLayout)]
enum ReqType {
    In = 0,
    #[expect(unused)]
    Out = 1,
}

pub struct VirtIOBlk<T: Transport> {
    transport: SpinMutex<T>,
    // 我们约定使用 transport 时和 requests 时需要同时持有 queue 的锁
    //
    // 所以 transport 和 requests 的锁其实是没必要的
    // transport 和 requests 上面还是要有锁是因为我们仍然希望能在不可变
    // 引用 VirtIOBlk 的情况下可变引用 transport 和 requests
    //
    // 我们这么做的原因是 rust 的 borrow checker 有点难搞
    queue: SpinMutex<VirtQueue<{ QUEUE_SIZE as usize }>>,
    condvar: Condvar,
    requests: SpinMutex<BTreeMap<u16, Condvar>>,
    /// 磁盘的容量，单位为字节
    capacity: u64,
}

impl<T: Transport> VirtIOBlk<T> {
    pub fn new(mut transport: T) -> Self {
        let mut features = BlkFeature::all();
        features.remove(BlkFeature::RO);
        features.remove(BlkFeature::SCSI);
        features.remove(BlkFeature::CONFIG_WCE);
        features.remove(BlkFeature::MQ);
        features.remove(BlkFeature::ANY_LAYOUT);
        features.remove(BlkFeature::RING_EVENT_IDX);
        features.remove(BlkFeature::RING_INDIRECT_DESC);
        transport.begin_init(features);
        let capacity = transport.read_config_consistent(|| {
            let low: u32 = transport.read_config_space(offset_of!(BlkConfig, capacity_low));
            let high: u32 = transport.read_config_space(offset_of!(BlkConfig, capacity_high));
            low as u64 | ((high as u64) << 32)
        }) * SECTOR_SIZE as u64;
        let queue = VirtQueue::new(&mut transport, 0);
        transport.finish_init();
        Self {
            transport: SpinMutex::new(transport, "VirtIOBlkTransport"),
            queue: SpinMutex::new(queue, "VirtIOBlkQueue"),
            condvar: Condvar::new(),
            requests: SpinMutex::new(BTreeMap::new(), "VirtIOBlkRequests"),
            capacity,
        }
    }

    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    /// 获取一个描述符，如果当前没有可用描述符则将当前线程挂起
    fn alloc_descriptor(&self) -> u16 {
        let mut queue = self.queue.lock();
        loop {
            if let Some(desc_idx) = queue.alloc_descriptor() {
                return desc_idx;
            }
            queue = self.condvar.wait(queue);
        }
    }

    /// 读取一个块的数据到 `buf` 中，并将当前线程挂起，直到数据读取完成
    ///
    /// # Preconditions
    ///
    /// `buf` 对应的内存地址必须满足虚拟内存地址和物理内存地址一致，并且
    /// 在物理内存中是连续的。否则可能会导致未定义行为。
    ///
    /// 为了满足这个条件，可以把 buf 分配到内核的堆上。
    ///
    /// # Panics
    ///
    /// - `buf` 的长度必须为一个扇区大小，即 512 字节，否则会 panic
    /// - 必须在线程中调用该函数，否则会 panic
    pub fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        assert_ne!(buf.len(), 0);
        assert!(buf.len() == SECTOR_SIZE);
        // req 要开在堆上，这样 req 的虚拟地址和物理地址一致，并且在物理内存中连续
        let req = Box::new(BlkReq {
            type_: ReqType::In,
            reserved: 0,
            sector: block_id as u64,
        });
        // 准备描述符
        let desc_idx3 = self.alloc_descriptor();
        let desc_idx2 = self.alloc_descriptor();
        let desc_idx1 = self.alloc_descriptor();
        // status 现在在栈上，需要将其地址转为物理地址
        let status: i8 = -1;
        let mut queue = self.queue.lock();
        queue.write_descriptor(
            desc_idx3,
            Descriptor::new(
                KERNEL_SPACE
                    .lock()
                    .translate_vaddr(VirtAddr::from_ref(&status))
                    .inner() as u64,
                1,
                DescFlags::WRITE,
                0,
            ),
        );
        queue.write_descriptor(
            desc_idx2,
            Descriptor::new(
                buf.as_ptr() as u64,
                buf.len() as u32,
                DescFlags::WRITE | DescFlags::NEXT,
                desc_idx3,
            ),
        );
        queue.write_descriptor(
            desc_idx1,
            Descriptor::new(
                Box::as_ptr(&req) as u64,
                size_of::<BlkReq>() as u32,
                DescFlags::NEXT,
                desc_idx2,
            ),
        );

        // 发送请求前先拿锁，防止请求很快就完成了，但是我们还没睡眠，然后就睡死了
        let mut requests = self.requests.lock();
        queue.request(desc_idx1, &mut *self.transport.lock());
        // 在睡眠之前需要先释放 requests 的锁，但是我们还要用到里面的 condvar
        // 所以这里将 request_condvar 泄露掉，同时强制对 requests 解锁
        // 但此时我们还持有对 request_condvar 的引用，此时就会存在一个竞争
        // 但我们现在仍持有 queue 锁，所以是安全的
        unsafe {
            requests.leak();
        }
        let request_condvar = requests.entry(desc_idx1).or_insert(Condvar::new());
        unsafe { self.requests.unlock() };
        // 我们后面用不到 queue，不需要再拿回锁
        request_condvar.wait(queue);
        // 后面我们被唤醒就说明请求已经完成了，所以不需要再进行额外判断

        if status != 0 {
            panic!("VirtIOBlk: read block failed with status {}", status);
        }
    }

    pub fn handle_interrupt(&self) {
        let mut queue = self.queue.lock();
        // VirtIO 只有在我们应答中断之后才会继续发新的中断
        // 应答中断并不意味着完成中断处理
        //
        // 我们应该先应答中断而不是在最后应答中断，不然可能出现：在我们处理完所有完成的请求
        // 到完成中断应答过程中，磁盘又完成了新的请求，但是由于我们还没有应答中断，导致该
        // 请求的完成没有触发中断，从而导致该请求的完成并没有被我们及时处理
        //
        // 如果我们先应答中断，再处理完成的请求，那么可能在我们处理到一半的时候又有新的请求
        // 到来了，于是我们就会继续处理。唯一带来的不太好的地方是，我们相当于是在当前中断
        // 处理过程中把后续中断要处理的请求给处理了，那么后续的中断可能没有处理任何请求。
        // 但这是没坏处的。
        self.transport.lock().ack_interrupt();
        while let Some(desc_idx) = queue.recycle_descriptor() {
            self.condvar.notify_all();
            self.requests
                .lock()
                .get(&desc_idx)
                .expect("VirtIOBlk: request not found")
                .notify_all();
        }
    }
}
