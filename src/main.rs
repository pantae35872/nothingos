#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![test_runner(nothingos::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate spin;
extern crate alloc;
extern crate core;

use core::panic::PanicInfo;
use alloc::string::String;
use bootloader::BootInfo;
use nothingos::println;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    nothingos::test_panic_handler(info)
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[no_mangle]
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    nothingos::init(boot_info);
    let test_string = String::from("ABXA");
    println!("{}", test_string.as_str());
    #[cfg(test)]
    test_main();
    hlt_loop(); 
}
