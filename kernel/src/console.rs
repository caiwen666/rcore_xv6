use crate::{driver::UART0, sync::spin::SpinMutex};
use core::fmt::{self, Write};

static GLOBAL_PRINT_LOCK: SpinMutex<()> = SpinMutex::new((), "global_print_lock");

struct Stdout;

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
