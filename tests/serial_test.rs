#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(nothingos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use nothingos::serial_println;

#[no_mangle]
pub fn start(boot_info: u8) -> ! {
    test_main();
    loop {}
}

#[test_case]
fn print_serial() {
    serial_println!("Test serial");
}
