#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![reexport_test_harness_main = "test_main"]
#![test_runner(nothingos::test_runner)]

extern crate nothingos;

use multiboot2::BootInformationHeader;
use nothingos::println;

#[no_mangle]
pub fn start(multiboot_information_address: *const BootInformationHeader) -> ! {
    nothingos::init(multiboot_information_address);
    test_main();
    loop {}
}

#[test_case]
fn test_println() {
    println!("test_println output");
}
