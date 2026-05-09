use crate::driver::virtio::{
    queue::{DescFlags, Descriptor, VirtQueue},
    transport::Transport,
};
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
    transport: T,
    queue: VirtQueue<{ QUEUE_SIZE as usize }>,
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
            transport,
            queue,
            capacity,
        }
    }

    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    /// 读取一个块的数据到 `buf` 中
    ///
    /// 一直阻塞到数据读取完成。
    /// 注意，这个函数假定只在内核初始时调用，此时还没有启动任何进程，
    /// 并且只有 CPU0 在跑，并且没有开启时钟中断和外部中断\
    ///
    /// # Panics
    ///
    /// `buf` 的长度必须为一个扇区大小，即 512 字节，否则会 panic
    pub fn read_block_sync(&mut self, block_id: usize, buf: &mut [u8]) {
        assert_ne!(buf.len(), 0);
        assert!(buf.len() == SECTOR_SIZE);
        let req = BlkReq {
            type_: ReqType::In,
            reserved: 0,
            sector: block_id as u64,
        };
        // 由于是启动初期，并且是同步读取，读完了之后就回收描述符
        // 所以肯定能获取到三个描述符
        // 描述符三
        let desc_idx3 = self.queue.alloc_descriptor().unwrap();
        let status: i8 = -1;
        self.queue.write_descriptor(
            desc_idx3,
            Descriptor::new(&status as *const i8 as u64, 1, DescFlags::WRITE, 0),
        );
        // 描述符二
        let desc_idx2 = self.queue.alloc_descriptor().unwrap();
        self.queue.write_descriptor(
            desc_idx2,
            Descriptor::new(
                buf.as_ptr() as u64,
                buf.len() as u32,
                DescFlags::WRITE | DescFlags::NEXT,
                desc_idx3,
            ),
        );
        // 描述符一
        let desc_idx1 = self.queue.alloc_descriptor().unwrap();
        self.queue.write_descriptor(
            desc_idx1,
            Descriptor::new(
                &req as *const BlkReq as u64,
                size_of::<BlkReq>() as u32,
                DescFlags::NEXT,
                desc_idx2,
            ),
        );
        // 发送请求
        self.queue.request(desc_idx1, &mut self.transport);
        // 等待完成
        loop {
            if let Some(desc_idx) = self.queue.recycle_descriptor() {
                if desc_idx == desc_idx1 {
                    break;
                } else {
                    panic!("VirtIOBlk: recycle_descriptor returned unexpected descriptor index");
                }
            }
        }
        if status != 0 {
            panic!("VirtIOBlk: read block failed with status {}", status);
        }
    }
}
