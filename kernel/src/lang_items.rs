use crate::{
    driver::{self, sifive_test::ShutdownReason},
    println,
};
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        println!("Panicked: {}", info.message());
    }
    driver::SIFIVE_TEST.shutdown(ShutdownReason::Failure, 0);
}
