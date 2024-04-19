#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![reexport_test_harness_main = "test_main"]
#![test_runner(nothingos::test_runner)]

extern crate nothingos;

use nothingos::{println, BootInformation};

#[no_mangle]
pub fn start(multiboot_information_address: *mut BootInformation) -> ! {
    nothingos::init(multiboot_information_address);
    test_main();
    loop {}
}

#[test_case]
fn test_println() {
    println!("test_println output");
}
