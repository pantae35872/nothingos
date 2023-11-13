#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(nothingos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use nothingos::{print, println};

#[no_mangle]
pub fn start(boot_info: u8) -> ! {
    test_main();
    loop {}
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
