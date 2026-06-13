use crate::{
    driver::UART0,
    fs::file::{File, FileSeekMethod},
    sync::spin::SpinMutex,
};
use core::fmt::{self, Write};

static GLOBAL_PRINT_LOCK: SpinMutex<()> = SpinMutex::new((), "global_print_lock");

pub struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            UART0.put_sync(c as u8);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    let _guard = GLOBAL_PRINT_LOCK.lock();
    Stdout.write_fmt(args).unwrap();
}

impl File for Stdout {
    fn read(&self, _buf: &mut [u8]) -> Option<usize> {
        None
    }

    fn write(&self, buf: &[u8]) -> Option<usize> {
        for c in buf {
            UART0.put(*c);
        }
        Some(buf.len())
    }

    fn seek(&self, _method: FileSeekMethod) -> Option<usize> {
        None
    }
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?))
    };
}

#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?))
    };
}

pub struct Stdin;

impl File for Stdin {
    fn read(&self, buf: &mut [u8]) -> Option<usize> {
        for c in buf.iter_mut() {
            *c = UART0.get();
        }
        Some(buf.len())
    }

    fn write(&self, _buf: &[u8]) -> Option<usize> {
        None
    }

    fn seek(&self, _method: FileSeekMethod) -> Option<usize> {
        None
    }
}
