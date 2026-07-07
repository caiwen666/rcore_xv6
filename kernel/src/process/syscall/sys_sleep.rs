use core::time::Duration;

use syscall_macros::syscall;

use crate::{error::SystemError, process::sleep::sleep_with_interval};

#[syscall(name = "SYS_SLEEP", id = 7)]
fn sys_fork(args: [usize; 6]) -> Result<usize, SystemError> {
    let us = args[0];
    let interval = Duration::from_micros(us as u64);
    let remain = sleep_with_interval(interval)
        .map(|v| v.as_micros() as usize)
        .unwrap_or(0);
    Ok(remain)
}
