use crate::{
    arch::MMArch,
    driver::virtio::transport::{DeviceStatus, DeviceType, InterruptStatus, Transport},
    mm::{MemoryManagementArch, address::PhysAddr},
};
use core::{ops::Deref, ptr::NonNull};
use safe_mmio::{
    UniqueMmioPointer, field, field_shared,
    fields::{ReadPure, ReadPureWrite, WriteOnly},
};
use zerocopy::{FromBytes, IntoBytes};

const MAGIC_VALUE: u32 = 0x7472_6976;
const LEGACY_VERSION: u32 = 1;
const MODERN_VERSION: u32 = 2;
const CONFIG_SPACE_OFFSET: usize = 0x100;

/// The version of the VirtIO MMIO transport supported by a device.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum MmioVersion {
    /// Legacy MMIO transport with page-based addressing.
    Legacy = LEGACY_VERSION,
    /// Modern MMIO transport.
    Modern = MODERN_VERSION,
}

impl TryFrom<u32> for MmioVersion {
    type Error = u32;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            LEGACY_VERSION => Ok(MmioVersion::Legacy),
            MODERN_VERSION => Ok(MmioVersion::Modern),
            _ => Err(value),
        }
    }
}

/// MMIO 设备寄存器接口（同时涵盖 legacy 与现代两种）。
///
/// 参考：4.2.2 MMIO Device Register Layout 与 4.2.4 Legacy interface
#[derive(Debug)]
#[repr(C)]
pub struct VirtIOHeader {
    /// 魔数 (0x0)
    magic: ReadPure<u32>,

    /// 设备版本号 (0x4)
    version: ReadPure<u32>,

    /// 设备 ID (0x8)
    device_id: ReadPure<u32>,

    /// 厂商 ID (0xc)
    vendor_id: ReadPure<u32>,

    /// 设备特性 (0x10)
    device_features: ReadPure<u32>,

    /// 保留 (0x14 ~ 0x1c)
    __r1: [u32; 3],

    /// 驱动特性 (0x20)
    driver_features: WriteOnly<u32>,

    /// 保留 (0x24)
    __r2: u32,

    /// 宿主页大小 (0x28)
    ///
    /// 初始化期间、尚未使用任何队列之前，驱动向该寄存器写入宿主机页大小（字节）。
    /// 该值应为 2 的幂
    legacy_guest_page_size: WriteOnly<u32>,

    /// 保留 (0x2c)
    __r3: u32,

    /// 虚拟队列索引 (0x30)
    ///
    /// 写入本寄存器可选择后续对 QueueNumMax、QueueNum、QueueAlign 与 QueuePFN
    /// 等寄存器的读写所针对的虚拟队列。首个队列的索引为 0（0x0）。
    queue_sel: WriteOnly<u32>,

    /// 最大队列长度 (0x34)
    queue_num_max: ReadPure<u32>,

    /// 虚拟队列长度 (0x38)
    ///
    /// 队列长度即队列中的元素个数。写入本寄存器告知驱动将使用的队列大小。
    /// 针对通过 QueueSel 选中的队列。
    queue_num: WriteOnly<u32>,

    /// used ring 对齐 (0x3c)
    ///
    /// 单位字节，针对通过 QueueSel 选中的队列。
    legacy_queue_align: WriteOnly<u32>,

    /// 虚拟队列的来宾物理页号 (0x40)
    ///
    /// 写入本寄存器告知设备虚拟队列位于宿主机物理地址空间中的位置。该值为从
    /// 队列描述符表起始页起的页索引。针对通过 QueueSel 选中的队列。
    legacy_queue_pfn: ReadPureWrite<u32>,

    /// 队列就绪 (0x44)
    queue_ready: ReadPureWrite<u32>,

    /// 保留 (0x48 ~ 0x4c)
    __r4: [u32; 2],

    /// 队列通知 (0x50)
    queue_notify: WriteOnly<u32>,

    /// 保留 (0x54 ~ 0x5c)
    __r5: [u32; 3],

    /// 中断状态 (0x60)
    interrupt_status: ReadPure<u32>,

    /// 中断确认 (0x64)
    interrupt_ack: WriteOnly<u32>,

    /// 保留 (0x68 ~ 0x6c)
    __r6: [u32; 2],

    /// 设备状态 (0x70)
    ///
    /// 读该寄存器返回当前设备状态标志。写入非零值会置位相应状态，表示 OS/驱动的
    /// 初始化进展。写入 0（0x0）会触发设备复位；设备会将所有队列的 QueuePFN 置为
    /// 0（0x0）。另见规范 3.1 Device Initialization。
    status: ReadPureWrite<DeviceStatus>,

    /// 保留 (0x74 ~ 0xf8)
    __r7: [u32; 34],

    /// 配置版本号 (0xfc)
    config_gerneration: ReadPure<u32>,
}

#[derive(Debug)]
pub struct MmioTransport<'a> {
    header: UniqueMmioPointer<'a, VirtIOHeader>,
    version: MmioVersion,
    device_type: DeviceType,
    config_space: UniqueMmioPointer<'a, [u8]>,
}

impl<'a> MmioTransport<'a> {
    /// # Panics
    ///
    /// - 该函数会检查 Magic 值是否正确，如果不正确则 panic
    /// - 如果出现了未知设备类型，则 panic
    /// - 如果出现了未知 mmio 版本，则 panic
    ///
    /// # Safety
    ///
    /// - 每个 MMIO 必须时刻只有一个对应的 MmioTransport 实例
    /// - 指向的 MMIO 区域必须具有生命周期 'a
    pub unsafe fn new(header: NonNull<VirtIOHeader>, mmio_size: usize) -> Self {
        let Some(config_space_size) = mmio_size.checked_sub(CONFIG_SPACE_OFFSET) else {
            panic!("MMIO size is too small");
        };
        // SAFETY: 调用者已经保证了正确性
        let header = unsafe { UniqueMmioPointer::new(header) };
        let magic = field_shared!(header, magic).read();
        if magic != MAGIC_VALUE {
            panic!("Invalid magic value: {}", magic);
        }
        let device_type = field_shared!(header, device_id)
            .read()
            .try_into()
            .unwrap_or_else(|device_id| panic!("Unknown device id: {}", device_id));
        let version = field_shared!(header, version)
            .read()
            .try_into()
            .unwrap_or_else(|version| panic!("Unknown version: {}", version));

        let config_space = unsafe {
            let config_ptr = header
                .ptr()
                .cast::<u8>()
                .wrapping_byte_add(CONFIG_SPACE_OFFSET)
                .cast_mut();
            let nn = NonNull::new_unchecked(config_ptr);
            UniqueMmioPointer::new(NonNull::slice_from_raw_parts(nn, config_space_size))
        };

        Self {
            header,
            version,
            device_type,
            config_space,
        }
    }

    pub fn version(&self) -> MmioVersion {
        self.version
    }

    pub fn vendor_id(&self) -> u32 {
        field_shared!(self.header, vendor_id).read()
    }
}

unsafe impl Send for MmioTransport<'_> {}

impl Transport for MmioTransport<'_> {
    fn device_type(&self) -> DeviceType {
        self.device_type
    }

    fn read_device_features(&mut self) -> u64 {
        // 只取了低 32 位
        field_shared!(self.header, device_features).read() as u64
    }

    fn write_driver_features(&mut self, driver_features: u64) {
        field!(self.header, driver_features).write(driver_features as u32);
    }

    fn max_queue_size(&mut self, queue: u16) -> u32 {
        field!(self.header, queue_sel).write(queue.into());
        field_shared!(self.header, queue_num_max).read()
    }

    fn notify(&mut self, queue: u16) {
        field!(self.header, queue_notify).write(queue.into());
    }

    fn set_status(&mut self, status: DeviceStatus) {
        field!(self.header, status).write(status);
    }

    fn set_guest_page_size(&mut self, guest_page_size: u32) {
        match self.version {
            MmioVersion::Legacy => {
                field!(self.header, legacy_guest_page_size).write(guest_page_size);
            }
            MmioVersion::Modern => {
                // No-op, modern devices don't care.
            }
        }
    }

    fn queue_set(&mut self, queue: u16, size: u32, descriptors: PhysAddr) {
        let align = MMArch::PAGE_SIZE as u32;
        let pfn = descriptors.inner() / MMArch::PAGE_SIZE;
        field!(self.header, queue_sel).write(queue.into());
        field!(self.header, queue_num).write(size);
        field!(self.header, legacy_queue_align).write(align);
        field!(self.header, legacy_queue_pfn).write(pfn as u32);
    }

    fn ack_interrupt(&mut self) -> InterruptStatus {
        let interrupt = field_shared!(self.header, interrupt_status).read();
        if interrupt != 0 {
            field!(self.header, interrupt_ack).write(interrupt);
            InterruptStatus::from_bits_truncate(interrupt)
        } else {
            InterruptStatus::empty()
        }
    }

    fn read_config_generation(&self) -> u32 {
        field_shared!(self.header, config_gerneration).read()
    }

    fn read_config_space<T: FromBytes + IntoBytes>(&self, offset: usize) -> T {
        assert!(
            align_of::<T>() <= 4,
            "Driver expected config space alignment of {} bytes, but VirtIO only guarantees 4 byte alignment.",
            align_of::<T>()
        );
        assert!(offset.is_multiple_of(align_of::<T>()));
        if core::hint::unlikely(self.config_space.len() < offset + size_of::<T>()) {
            panic!("Config space too small.")
        }
        unsafe {
            let ptr = self.config_space.ptr().byte_add(offset).cast::<T>();
            self.config_space
                .deref()
                .child(NonNull::new(ptr.cast_mut()).unwrap())
                .read_unsafe()
        }
    }
}
