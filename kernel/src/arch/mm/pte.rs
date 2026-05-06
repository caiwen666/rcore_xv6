use crate::mm::{address::PhysAddr, mem_space::MemoryPermission, page_table::PageTableEntry};
use bitflags::*;

bitflags! {
    pub struct Sv39PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Sv39PTE {
    pub bits: u64,
}

impl Sv39PTE {
    pub fn flags(&self) -> Sv39PTEFlags {
        Sv39PTEFlags::from_bits(self.bits as u8).unwrap()
    }
}
impl PageTableEntry for Sv39PTE {
    fn new_leaf(paddr: PhysAddr, permission: MemoryPermission) -> Self {
        let mut sv39_flags = Sv39PTEFlags::empty();
        if permission.contains(MemoryPermission::Readable) {
            sv39_flags.set(Sv39PTEFlags::R, true);
        }
        if permission.contains(MemoryPermission::Writable) {
            sv39_flags.set(Sv39PTEFlags::W, true);
        }
        if permission.contains(MemoryPermission::Executable) {
            sv39_flags.set(Sv39PTEFlags::X, true);
        }
        if permission.contains(MemoryPermission::UserAccessible) {
            sv39_flags.set(Sv39PTEFlags::U, true);
        }
        sv39_flags.set(Sv39PTEFlags::V, true);
        Sv39PTE {
            bits: (((paddr.inner() as u64) >> 12) << 10) | sv39_flags.bits() as u64,
        }
    }

    fn new_non_leaf(paddr: PhysAddr, is_user: bool) -> Self {
        let mut sv39_flags = Sv39PTEFlags::V;
        if is_user {
            sv39_flags.set(Sv39PTEFlags::U, true);
        }
        Sv39PTE {
            bits: (((paddr.inner() as u64) >> 12) << 10) | sv39_flags.bits() as u64,
        }
    }

    fn is_valid(&self) -> bool {
        self.flags().contains(Sv39PTEFlags::V)
    }

    fn paddr(&self) -> PhysAddr {
        const PPN_MASK: u64 = (1u64 << 44) - 1;
        let ppn = (self.bits >> 10) & PPN_MASK;
        PhysAddr::new((ppn as usize) << 12)
    }
}
