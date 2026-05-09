pub mod mmio;

use crate::{
    arch::MMArch,
    mm::{MemoryManagementArch, address::PhysAddr},
};
use bitflags::{Flags, bitflags};
use core::{
    fmt::{self, Debug, Formatter},
    ops::BitAnd,
};
use zerocopy::{FromBytes, Immutable, IntoBytes};

#[derive(Copy, Clone, Default, Eq, PartialEq)]
pub struct InterruptStatus(u32);

bitflags! {
    impl InterruptStatus: u32 {
        /// Indicates that a virtqueue buffer was used
        const QUEUE_INTERRUPT = 1 << 0;

        /// Indicates that a virtio device changed its configuration state
        const DEVICE_CONFIGURATION_INTERRUPT = 1 << 1;
    }
}

/// The device status field. Writing 0 into this field resets the device.
#[derive(Copy, Clone, Default, Eq, PartialEq, FromBytes, IntoBytes, Immutable)]
pub struct DeviceStatus(u32);

impl Debug for DeviceStatus {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "DeviceStatus(")?;
        bitflags::parser::to_writer(self, &mut *f)?;
        write!(f, ")")?;
        Ok(())
    }
}

bitflags! {
    impl DeviceStatus: u32 {
        /// Indicates that the guest OS has found the device and recognized it
        /// as a valid virtio device.
        const ACKNOWLEDGE = 1;

        /// Indicates that the guest OS knows how to drive the device.
        const DRIVER = 2;

        /// Indicates that something went wrong in the guest, and it has given
        /// up on the device. This could be an internal error, or the driver
        /// didn’t like the device for some reason, or even a fatal error
        /// during device operation.
        const FAILED = 128;

        /// Indicates that the driver has acknowledged all the features it
        /// understands, and feature negotiation is complete.
        const FEATURES_OK = 8;

        /// Indicates that the driver is set up and ready to drive the device.
        const DRIVER_OK = 4;

        /// Indicates that the device has experienced an error from which it
        /// can’t recover.
        const DEVICE_NEEDS_RESET = 64;
    }
}

/// Types of virtio devices.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DeviceType {
    Block = 2,
}

impl TryFrom<u32> for DeviceType {
    type Error = u32;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            2 => Ok(DeviceType::Block),
            _ => Err(value),
        }
    }
}

pub trait Transport {
    /// 获取设备类型
    fn device_type(&self) -> DeviceType;

    /// 获取设备的特性
    fn read_device_features(&mut self) -> u64;

    /// 写入驱动这边可以接受的特性
    fn write_driver_features(&mut self, features: u64);

    /// 获取最大队列长度
    fn max_queue_size(&mut self, queue: u16) -> u32;

    /// 通知指定队列
    fn notify(&mut self, queue: u16);

    /// 设置设备状态
    fn set_status(&mut self, status: DeviceStatus);

    /// 设置宿主机页大小
    fn set_guest_page_size(&mut self, page_size: u32);

    /// 设置指定队列
    fn queue_set(&mut self, queue: u16, size: u32, descriptors: PhysAddr);

    /// 应答中断
    #[expect(unused)]
    fn ack_interrupt(&mut self) -> InterruptStatus;

    fn begin_init<F: Flags<Bits = u64> + BitAnd<Output = F>>(
        &mut self,
        supported_features: F,
    ) -> F {
        self.set_status(DeviceStatus::empty());
        self.set_status(DeviceStatus::ACKNOWLEDGE | DeviceStatus::DRIVER);

        let device_features_bits = self.read_device_features();
        let device_features = F::from_bits_truncate(device_features_bits);
        let negotiated_features = supported_features & device_features;
        self.write_driver_features(negotiated_features.bits());

        self.set_status(
            DeviceStatus::ACKNOWLEDGE | DeviceStatus::DRIVER | DeviceStatus::FEATURES_OK,
        );

        self.set_guest_page_size(MMArch::PAGE_SIZE as u32);

        negotiated_features
    }

    fn finish_init(&mut self) {
        self.set_status(
            DeviceStatus::ACKNOWLEDGE
                | DeviceStatus::DRIVER
                | DeviceStatus::FEATURES_OK
                | DeviceStatus::DRIVER_OK,
        );
    }

    fn read_config_generation(&self) -> u32;

    fn read_config_space<T: FromBytes + IntoBytes>(&self, offset: usize) -> T;

    /// 确保读配置时，配置没有被改变
    fn read_config_consistent<T>(&self, f: impl Fn() -> T) -> T {
        loop {
            let before = self.read_config_generation();
            let result = f();
            let after = self.read_config_generation();
            if before == after {
                return result;
            }
        }
    }
}
