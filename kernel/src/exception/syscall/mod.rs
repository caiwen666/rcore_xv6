pub mod mm;

pub struct SyscallHandle {
    pub id: usize,
    #[expect(unused)]
    pub name: &'static str,
    pub handle: fn([usize; 6]) -> isize,
}

pub struct SyscallTable {
    entries: [Option<&'static SyscallHandle>; Self::SIZE],
}

impl SyscallTable {
    const SIZE: usize = 512;
    pub fn get(&self, id: usize) -> Option<&'static SyscallHandle> {
        *self.entries.get(id)?
    }
}

static mut SYSCALL_TABLE: SyscallTable = SyscallTable {
    entries: [const { None }; SyscallTable::SIZE],
};

#[inline]
pub fn syscall_table() -> &'static SyscallTable {
    unsafe { &SYSCALL_TABLE }
}

pub fn init_syscall_table() {
    unsafe extern "C" {
        fn ssyscall_table();
        fn esyscall_table();
    }
    let start = ssyscall_table as *const () as usize;
    let end = esyscall_table as *const () as usize;
    let size = end - start;
    assert!(size.is_multiple_of(core::mem::size_of::<SyscallHandle>()));
    let count = size / core::mem::size_of::<SyscallHandle>();

    let handles = unsafe { core::slice::from_raw_parts(start as *const SyscallHandle, count) };
    for handle in handles {
        assert!(
            handle.id < SyscallTable::SIZE,
            "Syscall ID out of range: {}",
            handle.id
        );
        unsafe {
            let slot = &mut SYSCALL_TABLE.entries[handle.id];
            assert!(slot.is_none(), "Syscall ID already in use: {}", handle.id);
            *slot = Some(handle);
        }
    }
}
